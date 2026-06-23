extern crate alloc;

use alloc::format;
use core::ffi::c_char;
use log::{Level, Log, Metadata, Record};

pub struct Logger;

enum RawLogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        let level = match record.level() {
            Level::Error => RawLogLevel::Error as u32,
            Level::Warn => RawLogLevel::Warn as u32,
            Level::Info => RawLogLevel::Info as u32,
            Level::Debug => RawLogLevel::Debug as u32,
            // Zephyr doesn't have a separate trace, so fold that into debug.
            Level::Trace => RawLogLevel::Debug as u32,
        };

        let msg = format!("{}: {}\0", record.target(), record.args());
        unsafe {
            rust_log_message(level, msg.as_ptr() as *const c_char);
        }
    }

    // Flush not needed.
    fn flush(&self) {}    
}

unsafe extern "C" {
    fn rust_log_message(level: u32, msg: *const c_char);
}

pub static LOGGER: Logger = Logger;