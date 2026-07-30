#![cfg_attr(feature = "platform-zephyr", no_std)]

extern crate alloc;

use log;
#[cfg(feature = "platform-desktop")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(all(feature = "platform-desktop", feature = "platform-zephyr"))]
compile_error!("jetbeep-core features `platform-desktop` and `platform-zephyr` are mutually exclusive");

#[cfg(not(any(feature = "platform-desktop", feature = "platform-zephyr")))]
compile_error!("jetbeep-core requires one platform feature: `platform-desktop` or `platform-zephyr`");

#[cfg(feature = "platform-zephyr")]
pub mod c_bindings;

#[cfg(feature = "platform-zephyr")]
mod log_impl;

#[cfg(feature = "platform-zephyr")]
#[path = "workq_zephyr.rs"]
pub mod workq;

#[cfg(feature = "platform-desktop")]
pub mod workq;

#[cfg(feature = "platform-desktop")]
pub mod fs;

#[cfg(feature = "platform-zephyr")]
#[path = "fs_zephyr.rs"]
pub mod fs;

/// File-backed JKV read/write helpers (platform-agnostic; built on `fs` + `jkv`).
pub mod jkv_file;

/// Cross-platform app launch requests (e.g. enter the service menu).
pub mod app_launcher;

#[cfg(feature = "platform-desktop")]
pub mod bus;

#[cfg(feature = "platform-zephyr")]
#[path = "bus_zephyr.rs"]
pub mod bus;

pub mod error;

/// CRC-64/XZ checksums (platform-agnostic; `no_std`-compatible).
pub mod crc64;

pub mod generation;

#[cfg(any(feature = "platform-zephyr", test))]
mod device_settings_cache;

#[cfg(feature = "platform-desktop")]
pub mod executor;

#[cfg(feature = "platform-zephyr")]
#[path = "executor_zephyr.rs"]
pub mod executor;

#[cfg(feature = "platform-desktop")]
pub mod lvgl;

#[cfg(feature = "platform-zephyr")]
#[path = "lvgl_zephyr.rs"]
pub mod lvgl;

pub mod proto;

#[cfg(feature = "platform-desktop")]
mod agent;

#[cfg(feature = "simulator")]
pub mod simulator;

#[cfg(feature = "platform-desktop")]
pub mod lvgl_fs_driver;

#[cfg(feature = "platform-desktop")]
pub fn init(fs_root: Option<&str>) {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_millis()
        .init();
    if let Err(e) = fs::configure_mount_root(fs_root) {
        panic!("Failed to configure desktop filesystem root: {}", e);
    }
    log::info!("jetbeep-core initialized");
}

#[cfg(feature = "platform-zephyr")]
pub fn init(_fs_root: Option<&str>) {
    log::set_max_level(log::LevelFilter::Info);
    log::set_logger(&log_impl::LOGGER).unwrap();
}

#[cfg(feature = "simulator")]
pub fn init_simulator(config_path: &str, layout_override: Option<&str>) {
    let device_settings_path = resolve_device_settings_path(config_path);
    if device_settings_path != config_path {
        log::info!(
            "simulator: device settings (user_params) sourced from {}",
            device_settings_path
        );
    }
    bus::set_simulator_config_path(&device_settings_path);
    simulator::init(config_path, layout_override);
}

/// Pick the file that provides `device_settings.user_settings.user_params`.
///
/// The layout catalog and the device settings can come from different places:
/// when `--simulator-config` points at a layouts **directory** (multi-layout
/// catalog), that directory carries no device settings, so we fall back to a
/// sibling `simulator_config.json` next to the directory (e.g.
/// `apps/<app>/simulator_config.json`). This lets layout switching and live
/// `user_params` editing coexist. When `--simulator-config` is a single file,
/// that same file provides the device settings (legacy behaviour).
#[cfg(feature = "simulator")]
fn resolve_device_settings_path(config_path: &str) -> String {
    let path = std::path::Path::new(config_path);
    if path.is_dir() {
        if let Some(parent) = path.parent() {
            let sibling = parent.join("simulator_config.json");
            if sibling.is_file() {
                return sibling.to_string_lossy().into_owned();
            }
        }
    }
    config_path.to_string()
}

#[cfg(feature = "platform-desktop")]
pub fn init_agent(config_path: &str) {
    agent::init(config_path);
}

#[cfg(feature = "platform-desktop")]
pub fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock is before UNIX_EPOCH")
        .as_micros() as u64
}

#[cfg(feature = "platform-zephyr")]
pub fn unix_time() -> u64 {
    unsafe { c_bindings::unix_time_now() }
}

#[cfg(feature = "platform-zephyr")]
#[unsafe(no_mangle)]
pub extern "C" fn jetbeep_rust_init() {
    init(None);
    device_settings_cache::start();
}

#[cfg(feature = "platform-zephyr")]
#[unsafe(no_mangle)]
pub extern "C" fn rust_device_settings_set_expected_crc(has_crc: bool, crc: u32) {
    workq::submit(core::time::Duration::from_millis(0), move |_| {
        device_settings_cache::set_expected_crc(has_crc.then_some(crc));
    });
}
