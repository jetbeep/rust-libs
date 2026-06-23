extern crate alloc;

use log;
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

#[cfg(feature = "platform-desktop")]
pub mod bus;

#[cfg(feature = "platform-zephyr")]
#[path = "bus_zephyr.rs"]
pub mod bus;

pub mod error;

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
    bus::set_simulator_config_path(config_path);
    simulator::init(config_path, layout_override);
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
}
