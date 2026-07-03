//! Desktop bus (hardware) API.
//!
//! With the `simulator` feature, operations drive the locker simulator state.
//! Without it, all operations return an "unsupported" error immediately (stubs).

use crate::error::Error;
use crate::proto::DeviceSettings;
use crate::proto::bus::{
    ChargeStatus,
    LockStatus,
    ModemInfo as ProtoModemInfo,
    PollBatteryGetInfoResponse,
    NetworkRegistrationInfo,
    NetworkRegistrationStatus,
    PollModemGetInfoResponse,
    PollVersionInfoResponse,
    SignalInfo,
    SimInfo,
};
use crate::proto::keypad::ButtonType;
use futures::channel::mpsc;
use futures::stream::Stream;
pub use jkv::JkvKey;
pub use jkv::JkvValue;
use serde::Deserialize;
use std::pin::Pin;
use std::path::Path;
use std::sync::RwLock;
use std::task::{Context, Poll};

pub type LockStatuses = Vec<LockStatus>;
pub type ModemInfo = PollModemGetInfoResponse;
pub type BatteryInfo = PollBatteryGetInfoResponse;
pub type VersionInfo = PollVersionInfoResponse;

pub const SERVER_REQUEST_DEFAULT_REQUEST_TYPE: i32 = 0;
pub const SERVER_REQUEST_DEFAULT_TIMEOUT_MS: i32 = 30_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerRequestParams {
    pub request_type: i32,
    pub timeout_ms: i32,
}

impl Default for ServerRequestParams {
    fn default() -> Self {
        Self {
            request_type: SERVER_REQUEST_DEFAULT_REQUEST_TYPE,
            timeout_ms: SERVER_REQUEST_DEFAULT_TIMEOUT_MS,
        }
    }
}

fn jkv_key_to_json_key(key: &JkvKey) -> String {
    match key {
        JkvKey::Int(v) => v.to_string(),
        JkvKey::String(v) => v.clone(),
    }
}

fn jkv_to_json_value(value: &JkvValue) -> serde_json::Value {
    match value {
        JkvValue::Undefined => serde_json::Value::Null,
        JkvValue::Null => serde_json::Value::Null,
        JkvValue::Bool(v) => serde_json::Value::Bool(*v),
        JkvValue::Int(v) => serde_json::Value::Number(serde_json::Number::from(*v)),
        JkvValue::Float(v) => serde_json::Number::from_f64(*v as f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        JkvValue::String(v) => serde_json::Value::String(v.clone()),
        JkvValue::Collection(entries) => {
            let mut map = serde_json::Map::new();
            for (k, v) in entries {
                map.insert(jkv_key_to_json_key(k), jkv_to_json_value(v));
            }
            serde_json::Value::Object(map)
        }
        JkvValue::Array(items) => {
            serde_json::Value::Array(items.iter().map(jkv_to_json_value).collect())
        }
    }
}

fn json_to_jkv_value(value: serde_json::Value) -> Result<JkvValue, Error> {
    match value {
        serde_json::Value::Null => Ok(JkvValue::Null),
        serde_json::Value::Bool(v) => Ok(JkvValue::Bool(v)),
        serde_json::Value::Number(n) => {
            if let Some(v) = n.as_i64() {
                let value = i32::try_from(v).map_err(|_| Error {
                    code: -22,
                    message: format!("JSON integer out of range for JKV Int: {}", v),
                })?;
                return Ok(JkvValue::Int(value));
            }

            if let Some(v) = n.as_u64() {
                let value = i32::try_from(v).map_err(|_| Error {
                    code: -22,
                    message: format!("JSON integer out of range for JKV Int: {}", v),
                })?;
                return Ok(JkvValue::Int(value));
            }

            if let Some(v) = n.as_f64() {
                if !v.is_finite() || v.abs() > f32::MAX as f64 {
                    return Err(Error {
                        code: -22,
                        message: format!("JSON float out of range for JKV Float: {}", v),
                    });
                }

                return Ok(JkvValue::Float(v as f32));
            }

            Err(Error {
                code: -22,
                message: "invalid JSON number".to_string(),
            })
        }
        serde_json::Value::String(v) => Ok(JkvValue::String(v)),
        serde_json::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(json_to_jkv_value(item)?);
            }
            Ok(JkvValue::Array(out))
        }
        serde_json::Value::Object(map) => {
            let mut out = Vec::with_capacity(map.len());
            for (k, v) in map {
                let key = match k.parse::<i32>() {
                    Ok(i) => JkvKey::Int(i),
                    Err(_) => JkvKey::String(k),
                };
                out.push((key, json_to_jkv_value(v)?));
            }
            Ok(JkvValue::Collection(out))
        }
    }
}

async fn server_request_impl(request_type: i32, timeout_ms: i32, body: JkvValue) -> Result<JkvValue, Error> {
    let response = modem_request_json(
        jkv_to_json_value(&body),
        Some(crate::agent::client::ScriptTypeOrString::Type(
            crate::agent::client::AspScriptType::Ui,
        )),
        Some(request_type),
        Some(timeout_ms),
    )
        .await
        .map_err(|err| Error {
            code: -1,
            message: format!("server_request modem_request failed: {}", err),
        })?;

    json_to_jkv_value(response)
}

async fn modem_request_json(
    content: serde_json::Value,
    script_type: Option<crate::agent::client::ScriptTypeOrString>,
    request_type: Option<i32>,
    timeout_ms: Option<i32>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request = crate::agent::client::HttpRequestQuery::new(
        content,
        script_type,
        request_type,
    );

    match crate::agent::state::get_client() {
        Some(client) => client.http_request(&request, None, timeout_ms).await,
        None => Err(Box::new(Error {
            code: -1,
            message: "Agent client is not initialized".to_string(),
        })),
    }
}

pub struct BarcodeReceiver {
    inner: mpsc::UnboundedReceiver<String>,
}

impl Stream for BarcodeReceiver {
    type Item = String;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

#[cfg(feature = "simulator")]
pub struct KeypadReceiver {
    inner: mpsc::UnboundedReceiver<KeypadKey>,
}

#[cfg(feature = "simulator")]
impl Stream for KeypadReceiver {
    type Item = KeypadKey;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

#[derive(Debug, Deserialize)]
struct SingleFileSimulatorConfig {
    #[allow(dead_code)]
    #[serde(default)]
    lockers: Option<serde_json::Value>,
    #[serde(default)]
    device_settings: DeviceSettings,
}

// ── Simulator implementation ──────────────────────────────────────────

#[cfg(feature = "simulator")]
use crate::simulator::state;

#[cfg(feature = "simulator")]
static TEST_MODEM_INFO: std::sync::LazyLock<ModemInfo> = std::sync::LazyLock::new(|| ModemInfo {
    modem_info: Some(ProtoModemInfo {
        imei: "359759080000001".to_string(),
    }),
    sim_info: Some(SimInfo {
        initialized: true,
        imsi: "250990123456789".to_string(),
        iccid: "8988212345678901234".to_string(),
    }),
    network_reg_info: Some(NetworkRegistrationInfo {
        status: NetworkRegistrationStatus::NetworkRegHomeNetwork as i32,
        tracking_area_code: "1001".to_string(),
        cell_id: "A1B2C3".to_string(),
        reject_cause: None,
        operator_name: "Test Operator".to_string(),
        operator_name_short: "TST".to_string(),
        mcc_mnc: "25099".to_string(),
        band: 20,
        pdp_activated: true,
        ip_address: "10.0.0.42".to_string(),
    }),
    signal_info: Some(SignalInfo {
        rspr: 75,
        rspr_index: 25,
        rsrq: 12,
        rsrq_index: 18,
    }),
});

#[cfg(feature = "simulator")]
static TEST_BATTERY_INFO: std::sync::LazyLock<BatteryInfo> = std::sync::LazyLock::new(|| BatteryInfo {
    percent: 83,
    voltage: 3875,
    status: ChargeStatus::NotCharging as i32,
    firmware_version: "bms-test-1.0.0".to_string(),
    cell_voltage: vec![3868, 3872, 3880],
    temperature: 27.5,
});

#[cfg(feature = "simulator")]
static TEST_VERSION_INFO: std::sync::LazyLock<VersionInfo> = std::sync::LazyLock::new(|| VersionInfo {
    app_version: "3.1.0-dev".to_string(),
    board_revision: "F0106".to_string(),
});

#[cfg(feature = "simulator")]
static SIMULATOR_CONFIG_PATH: RwLock<Option<String>> = RwLock::new(None);

#[cfg(feature = "simulator")]
pub fn set_simulator_config_path(config_path: &str) {
    if let Ok(mut guard) = SIMULATOR_CONFIG_PATH.write() {
        *guard = Some(config_path.to_string());
    }
}

#[cfg(feature = "simulator")]
fn read_device_settings_from_config(config_path: &str) -> Result<DeviceSettings, Error> {
    let path = Path::new(config_path);

    if path.is_dir() {
        return Ok(DeviceSettings::default());
    }

    let json = std::fs::read_to_string(path).map_err(|err| Error {
        code: -2,
        message: format!("failed reading {}: {}", path.display(), err),
    })?;

    let mut json_value: serde_json::Value = serde_json::from_str(&json).map_err(|err| Error {
        code: -22,
        message: format!("failed parsing wrapped config {}: {}", path.display(), err),
    })?;

    normalize_user_params_json(&mut json_value)?;
    normalize_keypad_alphabet_enum_strings(&mut json_value);

    let root: SingleFileSimulatorConfig = serde_json::from_value(json_value).map_err(|err| Error {
        code: -22,
        message: format!("failed parsing wrapped config {}: {}", path.display(), err),
    })?;

    Ok(root.device_settings)
}

#[cfg(feature = "simulator")]
fn normalize_user_params_json(root: &mut serde_json::Value) -> Result<(), Error> {
    let Some(user_settings) = root
        .get_mut("device_settings")
        .and_then(|v| v.get_mut("user_settings"))
    else {
        return Ok(());
    };

    let Some(user_params_json) = user_settings
        .get("user_params_json")
        .cloned()
    else {
        return Ok(());
    };

    let encoded = jkv::to_vec_with_header(&user_params_json).map_err(|err| Error {
        code: -22,
        message: format!("failed encoding device_settings.user_settings.user_params_json as JKV: {}", err),
    })?;

    let bytes_json = serde_json::Value::Array(
        encoded
            .into_iter()
            .map(|b| serde_json::Value::Number(serde_json::Number::from(b)))
            .collect(),
    );

    if let Some(map) = user_settings.as_object_mut() {
        map.insert("user_params".to_string(), bytes_json);
        map.remove("user_params_json");
    }

    Ok(())
}

#[cfg(feature = "simulator")]
fn normalize_keypad_alphabet_enum_strings(root: &mut serde_json::Value) {
    let Some(alphabet) = root
        .get_mut("device_settings")
        .and_then(|v| v.get_mut("keypad_settings"))
        .and_then(|v| v.get_mut("alphabet"))
        .and_then(|v| v.as_array_mut())
    else {
        return;
    };

    for item in alphabet.iter_mut() {
        let Some(name) = item.as_str() else {
            continue;
        };

        let Some(bt) = ButtonType::from_str_name(name) else {
            continue;
        };

        *item = serde_json::Value::Number((bt as i32).into());
    }
}

#[cfg(feature = "simulator")]
pub use crate::simulator::state::KeypadKey;

#[cfg(feature = "simulator")]
pub async fn lock_open(board_id: u32, lock_id: u32) -> Result<(), Error> {
    state::lock_open(board_id, lock_id)
}

#[cfg(feature = "simulator")]
pub async fn lock_statuses_get(board_id: u32) -> Result<LockStatuses, Error> {
    state::lock_statuses_get(board_id)
}

#[cfg(feature = "simulator")]
pub async fn barcode_scanner_start() -> Result<(), Error> {
    state::scanner_start();
    Ok(())
}

#[cfg(feature = "simulator")]
pub async fn barcode_scanner_stop() -> Result<(), Error> {
    state::scanner_stop();
    Ok(())
}

#[cfg(feature = "simulator")]
pub async fn sleep() -> Result<(), Error> {
    log::info!("bus::sleep requested, exiting application");
    crate::workq::terminate_now(0)
}

#[cfg(feature = "simulator")]
pub async fn server_request(body: JkvValue) -> Result<JkvValue, Error> {
    server_request_ex(body, ServerRequestParams::default()).await
}

#[cfg(feature = "simulator")]
pub async fn server_request_ex(body: JkvValue, params: ServerRequestParams) -> Result<JkvValue, Error> {
    server_request_impl(params.request_type, params.timeout_ms, body).await
}

#[cfg(feature = "simulator")]
pub async fn modem_get_info() -> Result<ModemInfo, Error> {
    Ok(TEST_MODEM_INFO.clone())
}

#[cfg(feature = "simulator")]
pub async fn battery_get_info() -> Result<BatteryInfo, Error> {
    Ok(TEST_BATTERY_INFO.clone())
}

#[cfg(feature = "simulator")]
pub async fn version_info() -> Result<VersionInfo, Error> {
    Ok(TEST_VERSION_INFO.clone())
}

#[cfg(feature = "simulator")]
pub async fn get_device_settings() -> Result<DeviceSettings, Error> {
    let config_path = SIMULATOR_CONFIG_PATH
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .ok_or_else(|| Error {
            code: -134,
            message: "simulator config path is not initialized".to_string(),
        })?;

    read_device_settings_from_config(&config_path)
}

#[cfg(feature = "simulator")]
pub fn barcode_subscribe() -> BarcodeReceiver {
    let (tx, rx) = mpsc::unbounded();
    state::set_barcode_sender(tx);
    BarcodeReceiver { inner: rx }
}

#[cfg(feature = "simulator")]
pub fn barcode_unsubscribe() {
    state::barcode_unsubscribe();
}

#[cfg(feature = "simulator")]
pub fn keypad_subscribe() -> KeypadReceiver {
    KeypadReceiver {
        inner: state::keypad_subscribe(),
    }
}

#[cfg(feature = "simulator")]
pub fn keypad_unsubscribe() {
    state::keypad_unsubscribe();
}

// ── Stub implementation (no simulator) ────────────────────────────────

#[cfg(not(feature = "simulator"))]
fn unsupported(op: &str) -> Error {
    Error {
        code: -134,
        message: format!("bus::{} not available on desktop", op),
    }
}

#[cfg(not(feature = "simulator"))]
pub async fn lock_open(_board_id: u32, _lock_id: u32) -> Result<(), Error> {
    log::warn!("bus::lock_open stub called");
    Err(unsupported("lock_open"))
}

#[cfg(not(feature = "simulator"))]
pub async fn lock_statuses_get(_board_id: u32) -> Result<LockStatuses, Error> {
    log::warn!("bus::lock_statuses_get stub called");
    Err(unsupported("lock_statuses_get"))
}

#[cfg(not(feature = "simulator"))]
pub async fn barcode_scanner_start() -> Result<(), Error> {
    log::warn!("bus::barcode_scanner_start stub called");
    Err(unsupported("barcode_scanner_start"))
}

#[cfg(not(feature = "simulator"))]
pub async fn barcode_scanner_stop() -> Result<(), Error> {
    log::warn!("bus::barcode_scanner_stop stub called");
    Err(unsupported("barcode_scanner_stop"))
}

#[cfg(not(feature = "simulator"))]
pub async fn sleep() -> Result<(), Error> {
    log::info!("bus::sleep requested, exiting application");
    crate::workq::terminate_now(0)
}

#[cfg(not(feature = "simulator"))]
pub async fn server_request(body: JkvValue) -> Result<JkvValue, Error> {
    server_request_ex(body, ServerRequestParams::default()).await
}

#[cfg(not(feature = "simulator"))]
pub async fn server_request_ex(body: JkvValue, params: ServerRequestParams) -> Result<JkvValue, Error> {
    server_request_impl(params.request_type, params.timeout_ms, body).await
}

#[cfg(not(feature = "simulator"))]
pub async fn modem_get_info() -> Result<ModemInfo, Error> {
    log::warn!("bus::modem_get_info stub called");
    Err(unsupported("modem_get_info"))
}

#[cfg(not(feature = "simulator"))]
pub async fn battery_get_info() -> Result<BatteryInfo, Error> {
    log::warn!("bus::battery_get_info stub called");
    Err(unsupported("battery_get_info"))
}

#[cfg(not(feature = "simulator"))]
pub async fn version_info() -> Result<VersionInfo, Error> {
    log::warn!("bus::version_info stub called");
    Err(unsupported("version_info"))
}

#[cfg(not(feature = "simulator"))]
pub async fn get_device_settings() -> Result<DeviceSettings, Error> {
    log::warn!("bus::get_device_settings stub called");
    Err(unsupported("get_device_settings"))
}

#[cfg(not(feature = "simulator"))]
pub fn barcode_subscribe() -> BarcodeReceiver {
    log::warn!("bus::barcode_subscribe stub called");
    let (_tx, rx) = mpsc::unbounded();
    BarcodeReceiver { inner: rx }
}

#[cfg(not(feature = "simulator"))]
pub fn barcode_unsubscribe() {}
