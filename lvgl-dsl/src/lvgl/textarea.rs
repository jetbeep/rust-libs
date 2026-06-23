use core::ffi::c_char;

use crate::c_bindings;

use super::util::to_null_terminated;
use super::widget::{LvObj, Widget};

/// LVGL text area widget (`lv_textarea`).
///
/// Wraps an `lv_textarea_create`-allocated object and inherits all layout,
/// style, event, and state methods from the [`Widget`] trait.
///
/// Requires `CONFIG_LV_USE_TEXTAREA=y` in the LVGL/Kconfig configuration.
pub struct TextArea {
    obj: LvObj,
}

impl Widget for TextArea {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl TextArea {
    /// Creates a new text area widget as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory).
    pub fn new(parent: &impl Widget) -> TextArea {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_textarea_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_textarea_create returned null");
        }
        TextArea {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Sets the placeholder text shown when the text area is empty.
    ///
    /// LVGL copies the string internally, so `text` only needs to live
    /// for the duration of this call.
    pub fn placeholder_text(&self, text: &str) -> &Self {
        let c_string = to_null_terminated(text);
        // SAFETY: c_string is valid NUL-terminated; LVGL copies the string before it drops.
        unsafe {
            c_bindings::lv_textarea_set_placeholder_text(
                self.lv_obj().raw(),
                c_string.as_ptr() as *const c_char,
            );
        }
        self
    }

    /// Constrains the maximum number of characters the text area accepts.
    ///
    /// Pass `0` to remove the limit (LVGL default).
    pub fn max_length(&self, n: u32) -> &Self {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe { c_bindings::lv_textarea_set_max_length(self.lv_obj().raw(), n) }
        self
    }

    /// Switches between single-line and multi-line mode.
    ///
    /// When `true`, the text area acts like a single-line text field and
    /// the Enter key triggers `LvEventCode::Ready` instead of inserting a newline.
    pub fn one_line(&self, yes: bool) -> &Self {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe { c_bindings::lv_textarea_set_one_line(self.lv_obj().raw(), yes) }
        self
    }

    /// Enables or disables password mode.
    ///
    /// In password mode each character is replaced with a bullet `•` after a
    /// short delay, hiding the user's input.
    pub fn password_mode(&self, yes: bool) -> &Self {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe { c_bindings::lv_textarea_set_password_mode(self.lv_obj().raw(), yes) }
        self
    }

    /// Replaces the entire text area content with `text`.
    ///
    /// LVGL copies the string internally, so `text` only needs to live
    /// for the duration of this call.
    pub fn set_text(&self, text: &str) -> &Self {
        let c_string = to_null_terminated(text);
        // SAFETY: c_string is valid NUL-terminated; LVGL copies the string before it drops.
        unsafe {
            c_bindings::lv_textarea_set_text(
                self.lv_obj().raw(),
                c_string.as_ptr() as *const c_char,
            );
        }
        self
    }

    /// Returns the current text content of this text area.
    pub fn get_text(&self) -> alloc::string::String {
        // SAFETY: obj is non-null and valid; LVGL returns a pointer to its
        // internal buffer which stays valid as long as the widget is alive.
        let raw_ptr = unsafe { c_bindings::lv_textarea_get_text(self.lv_obj().raw()) };
        if raw_ptr.is_null() {
            return alloc::string::String::new();
        }
        // SAFETY: LVGL guarantees a valid NUL-terminated C string.
        unsafe { core::ffi::CStr::from_ptr(raw_ptr) }
            .to_string_lossy()
            .into_owned()
    }

    /// Reads the text from a raw `lv_obj_t *` pointer stored as a `usize`.
    ///
    /// Used by static LVGL callbacks that cannot hold a typed `&TextArea`
    /// reference.  The caller must guarantee that `ptr` is non-zero and points
    /// to a live `lv_textarea` object.
    ///
    /// # Safety
    /// `ptr` must be a valid, non-null `*mut lv_obj_t` for an `lv_textarea`
    /// widget that has not been freed.  LVGL must be running on a single
    /// thread (no concurrent access).
    pub unsafe fn text_from_raw_ptr(ptr: usize) -> alloc::string::String {
        if ptr == 0 {
            return alloc::string::String::new();
        }
        let raw = ptr as *mut c_bindings::lv_obj_t;
        // SAFETY: Caller guarantees `raw` is a valid live lv_textarea pointer.
        let cstr_ptr = unsafe { c_bindings::lv_textarea_get_text(raw) };
        if cstr_ptr.is_null() {
            return alloc::string::String::new();
        }
        // SAFETY: LVGL guarantees a valid NUL-terminated C string.
        unsafe { core::ffi::CStr::from_ptr(cstr_ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::screen::Screen;
    use crate::lvgl::textarea::TextArea;

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_emits_create_call() {
        let screen = parent();
        let ta = TextArea::new(&screen);
        let calls = spy_drain();
        // First call is Screen::active() → lv_screen_active, second is textarea create.
        let create = calls
            .iter()
            .find(|c| matches!(c, LvCall::TextAreaCreate { .. }));
        assert!(create.is_some(), "expected TextAreaCreate spy call");
        drop(ta);
    }

    #[test]
    fn placeholder_text_emits_spy() {
        let screen = parent();
        let ta = TextArea::new(&screen);
        spy_drain();
        ta.placeholder_text("Enter name");
        let calls = spy_drain();
        assert!(
            matches!(&calls[0], LvCall::TextAreaSetPlaceholder { text, .. }
                if text == b"Enter name\0"),
            "unexpected calls: {calls:?}"
        );
    }

    #[test]
    fn max_length_emits_spy() {
        let screen = parent();
        let ta = TextArea::new(&screen);
        spy_drain();
        ta.max_length(32);
        let calls = spy_drain();
        assert!(
            matches!(calls[0], LvCall::TextAreaSetMaxLength { max: 32, .. }),
            "unexpected calls: {calls:?}"
        );
    }

    #[test]
    fn one_line_emits_spy() {
        let screen = parent();
        let ta = TextArea::new(&screen);
        spy_drain();
        ta.one_line(true);
        let calls = spy_drain();
        assert!(
            matches!(calls[0], LvCall::TextAreaSetOneLine { en: true, .. }),
            "unexpected calls: {calls:?}"
        );
    }

    #[test]
    fn password_mode_emits_spy() {
        let screen = parent();
        let ta = TextArea::new(&screen);
        spy_drain();
        ta.password_mode(true);
        let calls = spy_drain();
        assert!(
            matches!(calls[0], LvCall::TextAreaSetPasswordMode { en: true, .. }),
            "unexpected calls: {calls:?}"
        );
    }

    #[test]
    fn set_text_emits_spy() {
        let screen = parent();
        let ta = TextArea::new(&screen);
        spy_drain();
        ta.set_text("hello");
        let calls = spy_drain();
        assert!(
            matches!(&calls[0], LvCall::TextAreaSetText { text, .. }
                if text == b"hello\0"),
            "unexpected calls: {calls:?}"
        );
    }

    #[test]
    fn methods_are_chainable() {
        let screen = parent();
        let ta = TextArea::new(&screen);
        // Verify the builder pattern — each method returns &Self.
        ta.placeholder_text("Type…")
            .max_length(64)
            .one_line(true)
            .password_mode(false)
            .set_text("");
    }
}
