//! Async wrappers for poll_api FFI commands from zephyr-libs/bus_common/poll_api.

use core::time::Duration;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::{String, ToString};
use core::ffi::{c_char, CStr};
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use futures::channel::oneshot;
use futures::stream::Stream;
use jkv;
pub use jkv::JkvValue;
use prost::Message;

use crate::error::Error;
use crate::{error, workq::submit};
use crate::c_bindings::*;
use crate::proto::DeviceSettings;
use crate::proto::bus::{
	LockStatus,
	PollBatteryGetInfoResponse,
	PollLockStatusesGetResponse,
	PollModemGetInfoResponse,
	PollServerRequestResponse,
	PollVersionInfoResponse,
};

pub type LockStatuses = alloc::vec::Vec<LockStatus>;
pub type ModemInfo = PollModemGetInfoResponse;
pub type BatteryInfo = PollBatteryGetInfoResponse;
pub type VersionInfo = PollVersionInfoResponse;

#[derive(Clone, PartialEq, Message)]
struct PollGetDeviceSettingsResponse {
	#[prost(message, optional, tag = "1")]
	pub settings: Option<DeviceSettings>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeypadKey {
	Digit(u8),
	Star,
	Hash,
	A,
	B,
	C,
	D,
}

impl KeypadKey {
	pub fn label(&self) -> &'static str {
		match self {
			Self::Digit(0) => "0",
			Self::Digit(1) => "1",
			Self::Digit(2) => "2",
			Self::Digit(3) => "3",
			Self::Digit(4) => "4",
			Self::Digit(5) => "5",
			Self::Digit(6) => "6",
			Self::Digit(7) => "7",
			Self::Digit(8) => "8",
			Self::Digit(9) => "9",
			Self::Digit(_) => "?",
			Self::Star => "*",
			Self::Hash => "#",
			Self::A => "A",
			Self::B => "B",
			Self::C => "C",
			Self::D => "D",
		}
	}
}

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

/* Accessed on Rust workq context (subscribe/unsubscribe from app task, send from scheduled workq task). */
static mut BARCODE_QUEUE: Option<VecDeque<String>> = None;
static mut BARCODE_WAKER: Option<Waker> = None;
static mut BARCODE_GEN: u64 = 0;
static mut BARCODE_ACTIVE_GEN: u64 = 0;
static mut KEYPAD_WAKER: Option<Waker> = None;
static mut KEYPAD_PENDING: Option<KeypadKey> = None;
static mut KEYPAD_GEN: u64 = 0;
static mut KEYPAD_ACTIVE_GEN: u64 = 0;

unsafe fn barcode_queue_pop_front() -> Option<String> {
	let queue_ptr = core::ptr::addr_of_mut!(BARCODE_QUEUE);
	if (*queue_ptr).is_none() {
		*queue_ptr = Some(VecDeque::new());
	}
	(*queue_ptr).as_mut().and_then(|q| q.pop_front())
}

unsafe fn barcode_queue_push_back(item: String) {
	let queue_ptr = core::ptr::addr_of_mut!(BARCODE_QUEUE);
	if (*queue_ptr).is_none() {
		*queue_ptr = Some(VecDeque::new());
	}
	if let Some(queue) = (*queue_ptr).as_mut() {
		queue.push_back(item);
	}
}

unsafe fn barcode_queue_reset() {
	let queue_ptr = core::ptr::addr_of_mut!(BARCODE_QUEUE);
	*queue_ptr = Some(VecDeque::new());
}

unsafe fn barcode_queue_clear() {
	let queue_ptr = core::ptr::addr_of_mut!(BARCODE_QUEUE);
	*queue_ptr = None;
}

unsafe fn barcode_waker_replace(waker: Waker) {
	let waker_ptr = core::ptr::addr_of_mut!(BARCODE_WAKER);
	*waker_ptr = Some(waker);
}

unsafe fn barcode_waker_wake_and_clear() {
	let waker_ptr = core::ptr::addr_of_mut!(BARCODE_WAKER);
	if let Some(waker) = (*waker_ptr).take() {
		waker.wake();
	}
}

unsafe fn keypad_waker_replace(waker: Waker) {
	let waker_ptr = core::ptr::addr_of_mut!(KEYPAD_WAKER);
	*waker_ptr = Some(waker);
}

unsafe fn keypad_waker_wake_and_clear() {
	let waker_ptr = core::ptr::addr_of_mut!(KEYPAD_WAKER);
	if let Some(waker) = (*waker_ptr).take() {
		waker.wake();
	}
}

fn keypad_key_from_button_type(button_type: i32) -> Option<KeypadKey> {
	match button_type {
		1 => Some(KeypadKey::Digit(1)),
		2 => Some(KeypadKey::Digit(2)),
		3 => Some(KeypadKey::Digit(3)),
		4 => Some(KeypadKey::Digit(4)),
		5 => Some(KeypadKey::Digit(5)),
		6 => Some(KeypadKey::Digit(6)),
		7 => Some(KeypadKey::Digit(7)),
		8 => Some(KeypadKey::Digit(8)),
		9 => Some(KeypadKey::Digit(9)),
		10 => Some(KeypadKey::Digit(0)),
		11 => Some(KeypadKey::Star),
		12 => Some(KeypadKey::Hash),
		101 => Some(KeypadKey::A),
		102 => Some(KeypadKey::B),
		103 => Some(KeypadKey::C),
		104 => Some(KeypadKey::D),
		_ => None,
	}
}

fn payload_from_parts(data: *const u8, size: usize) -> Option<alloc::vec::Vec<u8>> {
	if data.is_null() {
		if size == 0 {
			Some(alloc::vec::Vec::new())
		} else {
			None
		}
	} else {
		Some(unsafe { core::slice::from_raw_parts(data, size).to_vec() })
	}
}

pub struct BarcodeReceiver {
	generation: u64,
}

impl Stream for BarcodeReceiver {
	type Item = String;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		unsafe {
			if self.generation != BARCODE_ACTIVE_GEN {
				return Poll::Ready(None);
			}

			if let Some(item) = barcode_queue_pop_front() {
				return Poll::Ready(Some(item));
			}

			barcode_waker_replace(cx.waker().clone());
			Poll::Pending
		}
	}
}

pub struct KeypadReceiver {
	generation: u64,
}

impl Stream for KeypadReceiver {
	type Item = KeypadKey;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		unsafe {
			if self.generation != KEYPAD_ACTIVE_GEN {
				return Poll::Ready(None);
			}

			let pending_ptr = core::ptr::addr_of_mut!(KEYPAD_PENDING);
			if let Some(item) = (*pending_ptr).take() {
				return Poll::Ready(Some(item));
			}

			keypad_waker_replace(cx.waker().clone());
			Poll::Pending
		}
	}
}

/// Opens a lock asynchronously.
pub async fn lock_open(board_id: u32, lock_id: u32) -> Result<(), Error> {
	let (sender, receiver) = oneshot::channel::<Result<(), Error>>();
	unsafe {
		poll_cmd_lock_open(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			board_id,
			lock_id,
			Some(cb_simple),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

unsafe extern "C" fn cb_simple(error: *const jb_error_t, user_data: *mut core::ffi::c_void) {
	let sender = unsafe { Box::from_raw(user_data as *mut oneshot::Sender<Result<(), Error>>) };
    let error = error::from_jb_error(error);

	submit(Duration::from_millis(0), move |_| {
		if error.code == 0 {
			sender.send(Ok(())).ok();
		} else {
			sender.send(Err(error)).ok();
		}
	});
}

/// Gets lock statuses asynchronously.
pub async fn lock_statuses_get(board_id: u32) -> Result<LockStatuses, Error> {
	let (sender, receiver): (oneshot::Sender<Result<LockStatuses, Error>>, oneshot::Receiver<Result<LockStatuses, Error>>) = oneshot::channel();
	unsafe {
		poll_cmd_lock_statuses_get(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			board_id,
			Some(cb_lock_statuses),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

unsafe extern "C" fn cb_lock_statuses(error: *const jb_error_t, data: *const u8, size: usize, user_data: *mut core::ffi::c_void) {
	let sender = unsafe { Box::from_raw(user_data as *mut oneshot::Sender<Result<LockStatuses, Error>>) };
    let error = error::from_jb_error(error);
	let payload = if error.code == 0 {
		payload_from_parts(data, size)
	} else {
		None
	};

	submit(Duration::from_millis(0), move |_| {
		if error.code == 0 {
			let Some(payload) = payload else {
				sender
					.send(Err(Error {
						code: -22,
						message: "lock_statuses_get response payload is invalid".to_string(),
					}))
					.ok();
				return;
			};

			match PollLockStatusesGetResponse::decode(payload.as_slice()) {
				Ok(response) => {
					let mut statuses = alloc::vec::Vec::with_capacity(response.statuses.len());
					for raw_status in response.statuses {
						match LockStatus::try_from(raw_status) {
							Ok(status) => statuses.push(status),
							Err(_) => {
								sender
									.send(Err(Error {
										code: -22,
										message: format!("invalid LockStatus value in response: {}", raw_status),
									}))
									.ok();
								return;
							}
						}
					}

					sender.send(Ok(statuses)).ok();
				}
				Err(decode_err) => {
					sender
						.send(Err(Error {
							code: -22,
							message: format!("failed to decode PollLockStatusesGetResponse protobuf: {}", decode_err),
						}))
						.ok();
				}
			}
		} else {
			sender.send(Err(error)).ok();
		}
	});
}

/// Sends a server request asynchronously.
pub async fn server_request(body: JkvValue) -> Result<JkvValue, Error> {
	server_request_ex(body, ServerRequestParams::default()).await
}

/// Sends a server request asynchronously with explicit parameters.
pub async fn server_request_ex(body: JkvValue, params: ServerRequestParams) -> Result<JkvValue, Error> {
	let encoded_body = jkv::encode_with_header(&body).map_err(|encode_err| Error {
		code: -22,
		message: format!("failed to encode server_request body as JKV: {}", encode_err),
	})?;

	let (sender, receiver) = oneshot::channel::<Result<JkvValue, Error>>();
	unsafe {
		poll_cmd_server_request(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			params.request_type,
			params.timeout_ms,
			encoded_body.as_ptr(),
			encoded_body.len(),
			Some(cb_server_request),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

unsafe extern "C" fn cb_server_request(error: *const jb_error_t, data: *const u8, size: usize, user_data: *mut core::ffi::c_void) {
	let sender = unsafe { Box::from_raw(user_data as *mut oneshot::Sender<Result<JkvValue, Error>>) };
    let error = error::from_jb_error(error);
	let payload = if error.code == 0 {
		payload_from_parts(data, size)
	} else {
		None
	};

	submit(Duration::from_millis(0), move |_| {
		if error.code == 0 {
			let Some(payload) = payload else {
				sender
					.send(Err(Error {
						code: -22,
						message: "server_request response payload is invalid".to_string(),
					}))
					.ok();
				return;
			};

			match PollServerRequestResponse::decode(payload.as_slice()) {
				Ok(response) => {
					if response.error_code != 0 {
						sender
							.send(Err(Error {
								code: response.error_code,
								message: response.error_text,
							}))
							.ok();
						return;
					}

					match jkv::decode_with_header(response.body.as_slice()) {
						Ok(value) => {
							sender.send(Ok(value)).ok();
						}
						Err(decode_err) => {
							sender
								.send(Err(Error {
									code: -22,
									message: format!("failed to decode server_request response body as JKV: {}", decode_err),
								}))
								.ok();
						}
					}
				}
				Err(decode_err) => {
					sender
						.send(Err(Error {
							code: -22,
							message: format!("failed to decode PollServerRequestResponse protobuf: {}", decode_err),
						}))
						.ok();
				}
			}
		} else {
			sender.send(Err(error)).ok();
		}
	});
}

/// Starts barcode scanner asynchronously.
pub async fn barcode_scanner_start() -> Result<(), Error> {
	let (sender, receiver) = oneshot::channel::<Result<(), Error>>();
	unsafe {
		poll_cmd_barcode_scanner_start(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			Some(cb_simple),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

/// Stops barcode scanner asynchronously.
pub async fn barcode_scanner_stop() -> Result<(), Error> {
	let (sender, receiver) = oneshot::channel::<Result<(), Error>>();
	unsafe {
		poll_cmd_barcode_scanner_stop(
			i2c_jb_bus_get_bus(i2c_jb_bus_get())    ,
			Some(cb_simple),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

/// Sends POLL_CMD_SLEEP to mainboard asynchronously.
pub async fn sleep() -> Result<(), Error> {
	let (sender, receiver) = oneshot::channel::<Result<(), Error>>();
	unsafe {
		poll_cmd_sleep(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			Some(cb_simple),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

/// Subscribe to scanned barcode events.
///
/// Desktop-compatible semantics:
/// - each call creates a fresh receiver,
/// - the latest subscription replaces a previous one,
/// - `barcode_unsubscribe()` clears active sender.
pub fn barcode_subscribe() -> BarcodeReceiver {
	unsafe {
		BARCODE_GEN = BARCODE_GEN.wrapping_add(1);
		BARCODE_ACTIVE_GEN = BARCODE_GEN;
		barcode_queue_reset();
		barcode_waker_wake_and_clear();
	}
	BarcodeReceiver { generation: unsafe { BARCODE_ACTIVE_GEN } }
}

/// Unsubscribe from scanned barcode events.
pub fn barcode_unsubscribe() {
	unsafe {
		BARCODE_ACTIVE_GEN = 0;
		barcode_queue_clear();
		barcode_waker_wake_and_clear();
	}
}

pub fn keypad_subscribe() -> KeypadReceiver {
	unsafe {
		KEYPAD_GEN = KEYPAD_GEN.wrapping_add(1);
		KEYPAD_ACTIVE_GEN = KEYPAD_GEN;
		KEYPAD_PENDING = None;
		keypad_waker_wake_and_clear();
	}
	KeypadReceiver { generation: unsafe { KEYPAD_ACTIVE_GEN } }
}

pub fn keypad_unsubscribe() {
	unsafe {
		KEYPAD_ACTIVE_GEN = 0;
		KEYPAD_PENDING = None;
		keypad_waker_wake_and_clear();
	}
}

/// C entrypoint used by i2c_jb_bus scanned barcode handler.
#[unsafe(no_mangle)]
pub extern "C" fn rust_bus_barcode_emit(barcode: *const c_char) {
	if barcode.is_null() {
		return;
	}

	let barcode = unsafe { CStr::from_ptr(barcode) };
	let barcode = barcode.to_string_lossy().into_owned();

	submit(Duration::from_millis(0), move |_| {
		unsafe {
			if BARCODE_ACTIVE_GEN == 0 {
				return;
			}

			barcode_queue_push_back(barcode);
			barcode_waker_wake_and_clear();
		}
	});
}

/// C entrypoint used by i2c_jb_bus keypad event handler.
#[unsafe(no_mangle)]
pub extern "C" fn rust_bus_keypad_emit(button_type: i32, is_keypress: bool) {
	if !is_keypress {
		return;
	}

	let Some(key) = keypad_key_from_button_type(button_type) else {
		return;
	};

	submit(Duration::from_millis(0), move |_| {
		unsafe {
			if KEYPAD_ACTIVE_GEN == 0 {
				return;
			}

			let waker_ptr = core::ptr::addr_of_mut!(KEYPAD_WAKER);
			if let Some(waker) = (*waker_ptr).take() {
				KEYPAD_PENDING = Some(key);
				waker.wake();
			}
		}
	});
}

/// Gets modem info asynchronously.
pub async fn modem_get_info() -> Result<ModemInfo, Error> {
	let (sender, receiver) = oneshot::channel::<Result<ModemInfo, Error>>();
	unsafe {
		poll_cmd_modem_get_info(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			Some(cb_modem_info),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

unsafe extern "C" fn cb_modem_info(error: *const jb_error_t, data: *const u8, size: usize, user_data: *mut core::ffi::c_void) {
	let sender = unsafe { Box::from_raw(user_data as *mut oneshot::Sender<Result<ModemInfo, Error>>) };
    let error = error::from_jb_error(error);
	let payload = if error.code == 0 {
		payload_from_parts(data, size)
	} else {
		None
	};

	submit(Duration::from_millis(0), move |_| {
		if error.code == 0 {
			let Some(payload) = payload else {
				sender
					.send(Err(Error {
						code: -22,
						message: "modem_get_info response payload is invalid".to_string(),
					}))
					.ok();
				return;
			};

			match PollModemGetInfoResponse::decode(payload.as_slice()) {
				Ok(response) => {
					sender.send(Ok(response)).ok();
				}
				Err(decode_err) => {
					sender
						.send(Err(Error {
							code: -22,
							message: format!("failed to decode PollModemGetInfoResponse protobuf: {}", decode_err),
						}))
						.ok();
				}
			}
		} else {
			sender.send(Err(error)).ok();
		}
	});
}

/// Gets battery info asynchronously.
pub async fn battery_get_info() -> Result<BatteryInfo, Error> {
	let (sender, receiver) = oneshot::channel::<Result<BatteryInfo, Error>>();
	unsafe {
		poll_cmd_battery_get_info(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			Some(cb_battery_info),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

unsafe extern "C" fn cb_battery_info(error: *const jb_error_t, data: *const u8, size: usize, user_data: *mut core::ffi::c_void) {
	let sender = unsafe { Box::from_raw(user_data as *mut oneshot::Sender<Result<BatteryInfo, Error>>) };
    let error = error::from_jb_error(error);
	let payload = if error.code == 0 {
		payload_from_parts(data, size)
	} else {
		None
	};

	submit(Duration::from_millis(0), move |_| {
		if error.code == 0 {
			let Some(payload) = payload else {
				sender
					.send(Err(Error {
						code: -22,
						message: "battery_get_info response payload is invalid".to_string(),
					}))
					.ok();
				return;
			};

			match PollBatteryGetInfoResponse::decode(payload.as_slice()) {
				Ok(response) => {
					sender.send(Ok(response)).ok();
				}
				Err(decode_err) => {
					sender
						.send(Err(Error {
							code: -22,
							message: format!("failed to decode PollBatteryGetInfoResponse protobuf: {}", decode_err),
						}))
						.ok();
				}
			}
		} else {
			sender.send(Err(error)).ok();
		}
	});
}

/// Gets version info asynchronously.
pub async fn version_info() -> Result<VersionInfo, Error> {
	let (sender, receiver) = oneshot::channel::<Result<VersionInfo, Error>>();
	unsafe {
		poll_cmd_version_info(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			Some(cb_version_info),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

unsafe extern "C" fn cb_version_info(error: *const jb_error_t, data: *const u8, size: usize, user_data: *mut core::ffi::c_void) {
	let sender = unsafe { Box::from_raw(user_data as *mut oneshot::Sender<Result<VersionInfo, Error>>) };
    let error = error::from_jb_error(error);
	let payload = if error.code == 0 {
		payload_from_parts(data, size)
	} else {
		None
	};

	submit(Duration::from_millis(0), move |_| {
		if error.code == 0 {
			let Some(payload) = payload else {
				sender
					.send(Err(Error {
						code: -22,
						message: "version_info response payload is invalid".to_string(),
					}))
					.ok();
				return;
			};

			match PollVersionInfoResponse::decode(payload.as_slice()) {
				Ok(response) => {
					sender.send(Ok(response)).ok();
				}
				Err(decode_err) => {
					sender
						.send(Err(Error {
							code: -22,
							message: format!("failed to decode PollVersionInfoResponse protobuf: {}", decode_err),
						}))
						.ok();
				}
			}
		} else {
			sender.send(Err(error)).ok();
		}
	});
}

/// Gets device settings asynchronously.
pub async fn get_device_settings() -> Result<DeviceSettings, Error> {
	let (sender, receiver) = oneshot::channel::<Result<DeviceSettings, Error>>();
	unsafe {
		poll_cmd_get_device_settings(
			i2c_jb_bus_get_bus(i2c_jb_bus_get()),
			Some(cb_device_settings),
			Box::into_raw(Box::new(sender)) as *mut _,
		);
	}
	receiver.await.unwrap()
}

unsafe extern "C" fn cb_device_settings(error: *const jb_error_t, data: *const u8, size: usize, user_data: *mut core::ffi::c_void) {
	let sender = unsafe { Box::from_raw(user_data as *mut oneshot::Sender<Result<DeviceSettings, Error>>) };
    let error = error::from_jb_error(error);
    let payload = if error.code == 0 {
        if data.is_null() || size == 0 {
            None
        } else {
            Some(unsafe { core::slice::from_raw_parts(data, size).to_vec() })
        }
    } else {
        None
    };

	submit(Duration::from_millis(0), move |_| {
		if error.code == 0 {
			let Some(payload) = payload else {
				sender
					.send(Err(Error {
						code: -22,
						message: "get_device_settings response payload is empty".to_string(),
					}))
					.ok();
				return;
			};

			match PollGetDeviceSettingsResponse::decode(payload.as_slice()) {
				Ok(response) => {
					if let Some(settings) = response.settings {
						sender.send(Ok(settings)).ok();
					} else {
						sender
							.send(Err(Error {
								code: -22,
								message: "PollGetDeviceSettingsResponse.settings is missing".to_string(),
							}))
							.ok();
					}
				}
				Err(decode_err) => {
					sender
						.send(Err(Error {
							code: -22,
							message: format!("failed to decode PollGetDeviceSettingsResponse protobuf: {}", decode_err),
						}))
						.ok();
				}
			}
		} else {
			sender.send(Err(error)).ok();
		}
	});
}
