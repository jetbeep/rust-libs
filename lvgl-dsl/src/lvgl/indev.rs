//! Input-device tuning shared by every screen.
//!
//! LVGL keeps these thresholds per `lv_indev_t`, and the platform entry point
//! registers the devices before the app runs — so an app can only adjust them
//! by walking the list after init.

use crate::c_bindings;

/// LVGL's built-in long-press threshold (`LV_INDEV_DEF_LONG_PRESS_TIME`).
pub const DEFAULT_LONG_PRESS_TIME_MS: u16 = 400;

/// Sets the long-press threshold on every registered input device.
///
/// The default 400 ms is tuned for a phone. On a kiosk panel an ordinary tap
/// regularly exceeds it whenever the UI thread stalls (a network response
/// being deserialized, a list being rebuilt): LVGL then reports the tap as a
/// long press, the keyboard opens its accent popup and the letter is never
/// typed.
///
/// # Safety
///
/// Must be called on the LVGL thread, and not from inside an input-device read
/// or event callback. This walks and mutates LVGL's global input-device list,
/// which LVGL itself reads unsynchronized from `lv_timer_handler`.
pub unsafe fn set_long_press_time(ms: u16) {
    // SAFETY: the caller guarantees LVGL-thread affinity. The list is walked
    // with LVGL's own iterator, so every non-null pointer it yields is a live
    // input device.
    unsafe {
        let mut indev = c_bindings::lv_indev_get_next(core::ptr::null_mut());
        while !indev.is_null() {
            c_bindings::lv_indev_set_long_press_time(indev, ms);
            indev = c_bindings::lv_indev_get_next(indev);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_long_press_time_reaches_every_registered_indev() {
        // SAFETY: the mock input-device list is thread-local to this test.
        unsafe { set_long_press_time(900) };
        assert_eq!(c_bindings::long_press_time_for_test(), 900);
    }

    #[test]
    fn walking_an_empty_indev_list_is_a_no_op() {
        c_bindings::set_active_indev_for_test(core::ptr::null_mut());
        // SAFETY: the mock input-device list is thread-local to this test.
        unsafe { set_long_press_time(1234) };
        assert_ne!(c_bindings::long_press_time_for_test(), 1234);
        c_bindings::set_active_indev_for_test(0x1000_0000 as *mut c_bindings::lv_indev_t);
    }
}
