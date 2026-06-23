use core::ptr;

use crate::c_bindings;

use super::screen_anim::ScreenAnim;
use super::widget::{LvObj, Widget};

pub struct Screen {
    obj: LvObj,
}

impl Widget for Screen {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl Screen {
    /// Returns a handle to the currently active LVGL screen.
    pub fn active() -> Screen {
        // SAFETY: `lv_screen_active` returns the active screen — panics if null.
        let obj = unsafe { c_bindings::lv_screen_active() };
        if obj.is_null() {
            panic!("lv_screen_active returned null");
        }
        Screen {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Creates a new blank screen (no parent — screens are top-level objects).
    pub fn new() -> Screen {
        // SAFETY: null parent creates a new screen object.
        let obj = unsafe { c_bindings::lv_obj_create(ptr::null_mut()) };
        if obj.is_null() {
            panic!("lv_obj_create returned null");
        }
        Screen {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Loads this screen instantly, replacing the currently active screen.
    pub fn load(&self) {
        // SAFETY: obj is non-null at construction.
        unsafe { c_bindings::lv_screen_load(self.lv_obj().raw()) }
    }

    /// Loads this screen with an animation.
    ///
    /// # Safety note
    /// When `auto_delete = true`, LVGL deletes the **old** screen and all its
    /// children from the C side after the transition. Any Rust handles to
    /// widgets on the previous screen become dangling pointers. This is
    /// inherent to LVGL's C API — do not access old-screen widget handles
    /// after calling this.
    pub fn load_anim(&self, anim: ScreenAnim, duration_ms: u32, delay_ms: u32, auto_delete: bool) {
        unsafe {
            c_bindings::lv_screen_load_anim(
                self.lv_obj().raw(),
                anim as u32,
                duration_ms,
                delay_ms,
                auto_delete,
            )
        }
    }

    /// Loads an LVGL screen given its raw `lv_obj_t *` address stored as a
    /// `usize` (obtained via [`Widget::raw_ptr`]).
    ///
    /// # Safety
    /// `ptr` must be a valid, non-null `lv_obj_t *` pointer to a live LVGL
    /// screen object that has not been deleted.  A null pointer is caught by
    /// `debug_assert!` in debug builds; in release builds it is undefined
    /// behaviour passed through to `lv_screen_load_anim`.
    pub unsafe fn load_ptr(
        ptr: usize,
        anim: ScreenAnim,
        duration_ms: u32,
        delay_ms: u32,
        auto_delete: bool,
    ) {
        debug_assert!(ptr != 0, "Screen::load_ptr called with null pointer");
        unsafe {
            c_bindings::lv_screen_load_anim(
                ptr as *mut c_bindings::lv_obj_t,
                anim as u32,
                duration_ms,
                delay_ms,
                auto_delete,
            )
        }
    }
}
