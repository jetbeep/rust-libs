//! Cross-platform app launch requests.
//!
//! Lets any app ask the platform to soft-kill the currently running app and
//! start another one (e.g. enter the service menu from the selected app).
//! Launch targets use the same integer values as the platform launch path
//! (`APP_LAUNCH_TARGET_*` in screen.git).
//!
//! * **Zephyr**: forwards to `rust_workq_request_launch_from_app()` in
//!   `app/src/rust-bridge/rust-workq.c`, which queues the proven
//!   `app_teardown() -> app_main(target)` sequence on the rust workq and
//!   keeps the current backlight brightness.
//! * **Desktop**: invokes a handler registered by the runner
//!   ([`set_launch_handler`]), deferred through the workq so the app is never
//!   torn down in the middle of an LVGL event callback.

use log;

/// Which app the platform should run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LaunchTarget {
    /// The build-selected (main) app.
    Selected = 0,
    /// The built-in service menu app.
    ServiceMenu = 1,
    /// The maintenance app.
    Maintenance = 2,
}

/// Request a soft-kill relaunch into `target`.
///
/// Safe to call from LVGL event callbacks: the actual teardown/launch is
/// always deferred (workq task on desktop, rust workq item on Zephyr).
#[cfg(feature = "platform-zephyr")]
pub fn request_launch(target: LaunchTarget) {
    log::info!("app_launcher: requesting launch of {:?}", target);
    unsafe { rust_workq_request_launch_from_app(target as core::ffi::c_int) };
}

#[cfg(feature = "platform-zephyr")]
unsafe extern "C" {
    /// Implemented in screen.git `app/src/rust-bridge/rust-workq.c`.
    /// Queues a soft-kill relaunch of `target` on the rust workq, keeping the
    /// current backlight brightness.
    fn rust_workq_request_launch_from_app(target: core::ffi::c_int);
}

/// Request a soft-kill relaunch into `target`.
///
/// Safe to call from LVGL event callbacks: the actual teardown/launch is
/// always deferred (workq task on desktop, rust workq item on Zephyr).
#[cfg(feature = "platform-desktop")]
pub fn request_launch(target: LaunchTarget) {
    log::info!("app_launcher: requesting launch of {:?}", target);
    crate::workq::submit(core::time::Duration::from_millis(0), move |_| {
        let handler = unsafe { *HANDLER.0.get() };
        match handler {
            Some(handler) => handler(target),
            None => log::warn!(
                "app_launcher: launch of {:?} requested but no handler registered",
                target
            ),
        }
    });
}

/// Register the runner callback that performs the actual app switch
/// (teardown of the current app + `app_main` of the new one).
///
/// Desktop only: on Zephyr the launch path lives in C (`rust-workq.c`).
/// Must be called once from the runner before any [`request_launch`].
#[cfg(feature = "platform-desktop")]
pub fn set_launch_handler(handler: fn(LaunchTarget)) {
    unsafe { *HANDLER.0.get() = Some(handler) };
}

#[cfg(feature = "platform-desktop")]
struct HandlerSlot(core::cell::UnsafeCell<Option<fn(LaunchTarget)>>);
// SAFETY: only accessed from the single LVGL/workq thread.
#[cfg(feature = "platform-desktop")]
unsafe impl Sync for HandlerSlot {}
#[cfg(feature = "platform-desktop")]
static HANDLER: HandlerSlot = HandlerSlot(core::cell::UnsafeCell::new(None));

/// Convenience wrapper: request a relaunch into the service menu app.
pub fn request_service_menu() {
    request_launch(LaunchTarget::ServiceMenu);
}
