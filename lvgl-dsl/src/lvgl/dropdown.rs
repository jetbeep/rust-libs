use core::ffi::c_char;

use crate::c_bindings;

use super::dropdown_dir::LvDropdownDir;
use super::util::to_null_terminated;
use super::widget::{LvObj, Widget};

/// LVGL dropdown widget (`lv_dropdown`).
///
/// Wraps an `lv_dropdown_create`-allocated object and inherits all layout,
/// style, event, and state methods from the [`Widget`] trait.
///
/// Requires `CONFIG_LV_USE_DROPDOWN=y` in the LVGL/Kconfig configuration.
pub struct Dropdown {
    obj: LvObj,
}

impl Widget for Dropdown {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl Dropdown {
    /// Creates a new dropdown widget as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory).
    pub fn new(parent: &impl Widget) -> Dropdown {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_dropdown_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_dropdown_create returned null");
        }
        Dropdown {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Sets the options list for the dropdown.
    ///
    /// `opts` must be a newline-delimited string of option labels,
    /// e.g. `"English\nFrench\nGerman"`.
    ///
    /// LVGL copies the string internally, so `opts` only needs to live
    /// for the duration of this call.
    pub fn options(&self, opts: &str) -> &Self {
        let c_string = to_null_terminated(opts);
        // SAFETY: c_string is valid NUL-terminated; LVGL copies the string before it drops.
        unsafe {
            c_bindings::lv_dropdown_set_options(
                self.lv_obj().raw(),
                c_string.as_ptr() as *const c_char,
            );
        }
        self
    }

    /// Selects the option at zero-based `index`.
    ///
    /// If `index` is out of range, LVGL clamps it to the last valid index.
    pub fn selected(&self, index: u16) -> &Self {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe { c_bindings::lv_dropdown_set_selected(self.lv_obj().raw(), index.into()) }
        self
    }

    /// Programmatically opens the dropdown list.
    pub fn open(&self) -> &Self {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe { c_bindings::lv_dropdown_open(self.lv_obj().raw()) }
        self
    }

    /// Programmatically closes the dropdown list.
    pub fn close(&self) -> &Self {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe { c_bindings::lv_dropdown_close(self.lv_obj().raw()) }
        self
    }

    /// Sets the direction in which the option list opens.
    ///
    /// Defaults to [`LvDropdownDir::Down`].
    pub fn direction(&self, dir: LvDropdownDir) -> &Self {
        // SAFETY: obj is non-null and valid; dir is a repr(u32) enum so its integer value is valid.
        unsafe { c_bindings::lv_dropdown_set_dir(self.lv_obj().raw(), dir as u32) }
        self
    }

    /// Constrains the maximum pixel height of the option list.
    ///
    /// Pass `0` to remove any height limit (LVGL default).
    ///
    /// Note: `lv_dropdown_set_max_height` was removed in LVGL v9.
    /// Height is applied via the style layer (`lv_obj_set_style_max_height`).
    pub fn max_height(&self, px: i32) -> &Self {
        Widget::max_height(self, super::size::Size::Px(px));
        self
    }

    /// Sets the symbol (icon string) shown on the dropdown button.
    ///
    /// Pass `""` to show no symbol — LVGL treats a `NULL` symbol pointer as
    /// "no symbol".  LVGL copies the string for any non-empty value, so
    /// `text` only needs to live for the duration of this call.
    pub fn symbol(&self, text: &str) -> &Self {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        // A null symbol pointer is the correct LVGL signal for "no symbol".
        if text.is_empty() {
            unsafe {
                c_bindings::lv_dropdown_set_symbol(self.lv_obj().raw(), core::ptr::null());
            }
        } else {
            let c_string = to_null_terminated(text);
            // SAFETY: c_string is valid NUL-terminated; LVGL copies the string before it drops.
            unsafe {
                c_bindings::lv_dropdown_set_symbol(
                    self.lv_obj().raw(),
                    c_string.as_ptr() as *const core::ffi::c_void,
                );
            }
        }
        self
    }

    /// Returns the zero-based index of the currently selected option.
    ///
    /// Defaults to `0` (the first option) until [`selected`](Dropdown::selected) is called.
    #[must_use]
    pub fn get_selected(&self) -> u16 {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe { c_bindings::lv_dropdown_get_selected(self.lv_obj().raw()) as u16 }
    }

    /// Returns the selected index from a raw `lv_obj_t *` pointer stored as a `usize`.
    ///
    /// Used by static LVGL callbacks that cannot hold a typed `&Dropdown` reference.
    /// The caller must guarantee that `ptr` is non-zero and points to a live
    /// `lv_dropdown` object.
    ///
    /// # Safety
    /// `ptr` must be a valid, non-null `*mut lv_obj_t` for an `lv_dropdown` widget
    /// that has not been freed. LVGL must be running on a single thread (no concurrent
    /// access).
    pub unsafe fn selected_from_raw_ptr(ptr: usize) -> u16 {
        if ptr == 0 {
            return 0;
        }
        let raw = ptr as *mut c_bindings::lv_obj_t;
        // SAFETY: Caller guarantees `raw` is a valid live lv_dropdown pointer.
        unsafe { c_bindings::lv_dropdown_get_selected(raw) as u16 }
    }
}

#[cfg(test)]
mod tests {
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::dropdown::Dropdown;
    use crate::lvgl::dropdown_dir::LvDropdownDir;
    use crate::lvgl::screen::Screen;

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_does_not_panic() {
        let p = parent();
        let _ = Dropdown::new(&p);
    }

    #[test]
    fn options_records_call() {
        let p = parent();
        let dd = Dropdown::new(&p);
        spy_drain();
        dd.options("A\nB\nC");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::DropdownSetOptions { options_bytes, .. } if options_bytes == b"A\nB\nC\0"
            )),
            "expected DropdownSetOptions with b\"A\\nB\\nC\\0\", got: {:?}",
            calls
        );
    }

    #[test]
    fn selected_records_call() {
        let p = parent();
        let dd = Dropdown::new(&p);
        spy_drain();
        dd.selected(2);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::DropdownSetSelected { index, .. } if *index == 2
            )),
            "expected DropdownSetSelected{{index: 2}}, got: {:?}",
            calls
        );
    }

    #[test]
    fn open_records_call() {
        let p = parent();
        let dd = Dropdown::new(&p);
        spy_drain();
        dd.open();
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::DropdownOpen { .. })),
            "expected DropdownOpen, got: {:?}",
            calls
        );
    }

    #[test]
    fn close_records_call() {
        let p = parent();
        let dd = Dropdown::new(&p);
        spy_drain();
        dd.close();
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::DropdownClose { .. })),
            "expected DropdownClose, got: {:?}",
            calls
        );
    }

    #[test]
    fn direction_records_call() {
        let p = parent();
        let dd = Dropdown::new(&p);
        spy_drain();
        dd.direction(LvDropdownDir::Up);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::DropdownSetDir { dir, .. } if *dir == LvDropdownDir::Up as u32
            )),
            "expected DropdownSetDir{{dir: 1}}, got: {:?}",
            calls
        );
    }

    #[test]
    fn max_height_records_call() {
        let p = parent();
        let dd = Dropdown::new(&p);
        spy_drain();
        dd.max_height(200);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::SetStyleMaxHeight { value: 200, .. })),
            "expected SetStyleMaxHeight {{ value: 200 }}, got: {:?}",
            calls
        );
    }

    #[test]
    fn symbol_records_call() {
        let p = parent();
        let dd = Dropdown::new(&p);
        spy_drain();
        dd.symbol("v");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::DropdownSetSymbol { symbol_bytes: Some(b), .. } if b == b"v\0"
            )),
            "expected DropdownSetSymbol with Some(b\"v\\0\"), got: {:?}",
            calls
        );
    }

    #[test]
    fn symbol_empty_passes_null() {
        let p = parent();
        let dd = Dropdown::new(&p);
        spy_drain();
        dd.symbol("");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::DropdownSetSymbol {
                    symbol_bytes: None,
                    ..
                }
            )),
            "expected DropdownSetSymbol with None (null), got: {:?}",
            calls
        );
    }

    #[test]
    fn chaining_returns_self() {
        let p = parent();
        let dd = Dropdown::new(&p);
        // All methods return &Self — this line must compile without error.
        dd.options("X\nY")
            .selected(0)
            .direction(LvDropdownDir::Down)
            .max_height(150)
            .symbol("▼");
    }

    #[test]
    fn get_selected_defaults_to_zero() {
        let p = parent();
        let dd = Dropdown::new(&p);
        assert_eq!(dd.get_selected(), 0);
    }

    #[test]
    fn get_selected_after_set() {
        let p = parent();
        let dd = Dropdown::new(&p);
        dd.selected(3);
        assert_eq!(dd.get_selected(), 3);
    }

    #[test]
    fn selected_from_raw_ptr_zero_returns_zero() {
        // ptr == 0 must return 0 without calling into LVGL bindings.
        let result = unsafe { Dropdown::selected_from_raw_ptr(0) };
        assert_eq!(result, 0);
    }
}
