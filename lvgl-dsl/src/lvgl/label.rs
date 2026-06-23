use core::ffi::c_char;

use crate::c_bindings;

use super::util::to_null_terminated;
use super::widget::{LvObj, Widget};

#[repr(u32)]
pub enum LvLabelLongMode {
    Wrap = c_bindings::LV_LABEL_LONG_WRAP,
    Dot = c_bindings::LV_LABEL_LONG_DOT,
    Scroll = c_bindings::LV_LABEL_LONG_SCROLL,
    ScrollCircular = c_bindings::LV_LABEL_LONG_SCROLL_CIRC,
    Clip = c_bindings::LV_LABEL_LONG_CLIP,
}

/// LVGL label widget (`lv_label`).
///
/// Wraps an `lv_label_create`-allocated object and inherits all layout,
/// style, event, and state methods from the [`Widget`] trait.  Use
/// [`text`](Label::text) to set or update the displayed string.
pub struct Label {
    obj: LvObj,
}

impl Widget for Label {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl Label {
    /// Creates a new label widget as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory).
    pub fn new(parent: &impl Widget) -> Label {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_label_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_label_create returned null");
        }
        Label {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Wraps a raw `*mut lv_obj_t` pointer in a `Label` without taking ownership semantics.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - `ptr` is non-null and points to a valid `lv_obj_t` of label widget kind
    ///   (i.e. created by `lv_label_create`).
    /// - The wrapper does not outlive the underlying C-owned LVGL object.
    /// - No other wrapper exists for the same pointer that would also attempt to free it
    ///   (LVGL retains ownership of the underlying object in typical usage).
    /// - The pointer is only used on the LVGL thread (LVGL is single-threaded).
    pub unsafe fn from_raw(ptr: *mut crate::c_bindings::lv_obj_t) -> Self {
        Label {
            obj: LvObj::from_raw(ptr),
        }
    }

    /// Enables or disables recolor markup (`#RRGGBB text#`) in the label text.
    /// Required for [`highlight_markup`](super::searchbar::highlight::highlight_markup)
    /// output to render its inline color spans.
    pub fn recolor(&self, en: bool) -> &Self {
        // SAFETY: `LvObj` instances are only constructed from non-null LVGL object pointers.
        unsafe { c_bindings::lv_label_set_recolor(self.lv_obj().raw(), en) };
        self
    }

    pub fn long_mode(&self, mode: LvLabelLongMode) -> &Self {
        // SAFETY: `LvObj` instances are only constructed from non-null LVGL object pointers.
        unsafe { c_bindings::lv_label_set_long_mode(self.lv_obj().raw(), mode as u32) };
        self
    }

    /// Sets the label text. LVGL copies the string — the temporary buffer is safe to drop.
    pub fn text(&self, t: &str) -> &Self {
        let c_string = to_null_terminated(t);
        // SAFETY: LVGL copies the string before `c_string` drops at end of scope.
        unsafe {
            c_bindings::lv_label_set_text(self.lv_obj().raw(), c_string.as_ptr() as *const c_char);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::label::Label;
    use crate::lvgl::screen::Screen;

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_does_not_panic() {
        let p = parent();
        let _ = Label::new(&p);
    }
    #[test]
    fn text_passes_null_terminated_bytes() {
        let p = parent();
        let lbl = Label::new(&p);
        spy_drain();
        lbl.text("hi");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"hi\0"
            )),
            "expected LabelSetText with b\"hi\\0\", got: {:?}",
            calls
        );
    }
    #[test]
    fn text_empty_string_sends_nul_byte() {
        let p = parent();
        let lbl = Label::new(&p);
        spy_drain();
        lbl.text("");
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c,
                LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"\0"
            )),
            "expected LabelSetText with b\"\\0\", got: {:?}",
            calls
        );
    }
}

#[cfg(test)]
mod pub_from_raw_tests {
    use super::*;

    #[test]
    fn pub_from_raw_round_trips_pointer() {
        let p = 0xDEAD_BEEF_usize as *mut crate::c_bindings::lv_obj_t;
        let l = unsafe { Label::from_raw(p) };
        assert_eq!(l.lv_obj().raw(), p);
        std::mem::forget(l);
    }
}
