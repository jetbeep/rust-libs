//! Custom LVGL filesystem driver that resolves paths relative to `--fs-root`.
//!
//! Registers drive letter **`J`** so that LVGL widgets can load assets with
//! paths like `"J:images/logo.png"`.  The base path is built at runtime from
//! the mount root configured via `--fs-root` (+ `lfs1/` subdirectory),
//! eliminating the compile-time `LV_FS_STDIO_PATH` dependency.
//!
//! The actual driver implementation lives in C (`lv_fs_j_register.c`) to
//! avoid Rust↔C callback ABI issues.  This module only configures and
//! triggers the registration.

use std::ffi::CString;
use std::os::raw::c_char;
use std::path::PathBuf;

extern "C" {
    fn lvgl_fs_j_set_base_path(base_path: *const c_char);
    fn lvgl_fs_j_register();
}

/// Initialise the `J:` LVGL filesystem driver.
///
/// Must be called **after** `lv_init()`.
///
/// `sub_path` is the subdirectory under `--fs-root` where LVGL assets live
/// (typically `"lfs1"`).  Pass `None` to default to `"lfs1/"`.
pub fn init(fs_root_resolved: &str, sub_path: Option<&str>) {
    let sub = sub_path.unwrap_or("lfs1");
    let mut base = PathBuf::from(fs_root_resolved);
    base.push(sub);

    let mut base_str = base.to_string_lossy().into_owned();
    if !base_str.ends_with('/') {
        base_str.push('/');
    }

    let c_base = CString::new(base_str.clone())
        .expect("fs root path must not contain NUL bytes");

    unsafe {
        lvgl_fs_j_set_base_path(c_base.as_ptr());
        lvgl_fs_j_register();
    }

    log::info!("LVGL J: driver base path: {}", base_str);
}
