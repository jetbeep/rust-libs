#![cfg_attr(not(any(test, no_zephyr)), no_std)]

extern crate alloc;

pub mod c_bindings;
mod lvgl;

pub use c_bindings::lv_anim_t;
pub use c_bindings::lv_color_t;
pub use c_bindings::lv_font_t;
pub use c_bindings::lv_obj_t;
pub use c_bindings::{
    LV_ANIM_REPEAT_INFINITE, lv_anim_delete, lv_anim_init, lv_anim_set_completed_cb,
    lv_anim_set_duration, lv_anim_set_exec_cb, lv_anim_set_values, lv_anim_set_var, lv_anim_start,
};
pub use c_bindings::lv_mem_monitor_t;
pub use lvgl::*;

/// Desktop-sim spy hooks re-exported for examples and downstream
/// integration tests. Only available when building against the mock
/// LVGL backend (i.e., not the real Zephyr or desktop-sim CMake build).
#[cfg(any(test, all(no_zephyr, not(desktop_sim))))]
pub mod test_support {
    pub use crate::c_bindings::{
        LV_EVENT_CLICKED, LV_EVENT_VALUE_CHANGED, SpyFixture, set_next_scroll_bottom,
        spy_emit_event,
    };
}
