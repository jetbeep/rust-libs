use core::{
    cell::Cell,
    ffi::{CStr, c_char},
};

use crate::c_bindings;

use super::widget::{LvObj, Widget};

pub const BUTTONMATRIX_BUTTON_NONE: u32 = 0xFFFF;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct ButtonMatrixMapEntry(*const c_char);

impl ButtonMatrixMapEntry {
    pub const fn new(s: &'static CStr) -> Self {
        ButtonMatrixMapEntry(s.as_ptr())
    }
}

// SAFETY: entries are constructed from immutable &'static CStr values, and this wrapper never
// writes through the stored pointer.
unsafe impl Sync for ButtonMatrixMapEntry {}
// SAFETY: entries are constructed from immutable &'static CStr values, and this wrapper never
// writes through the stored pointer.
unsafe impl Send for ButtonMatrixMapEntry {}

pub type ButtonMatrixMap = [ButtonMatrixMapEntry];
pub type ButtonMatrixCtrlMap = [u32];

pub const BUTTONMATRIX_CTRL_W1: u32 = 1;
pub const BUTTONMATRIX_CTRL_W2: u32 = 2;
pub const BUTTONMATRIX_CTRL_W3: u32 = 3;
pub const BUTTONMATRIX_CTRL_W4: u32 = 4;
pub const BUTTONMATRIX_CTRL_W5: u32 = 5;
pub const BUTTONMATRIX_CTRL_W6: u32 = 6;
pub const BUTTONMATRIX_CTRL_W7: u32 = 7;
pub const BUTTONMATRIX_CTRL_W8: u32 = 8;
pub const BUTTONMATRIX_CTRL_W9: u32 = 9;
pub const BUTTONMATRIX_CTRL_W10: u32 = 10;
pub const BUTTONMATRIX_CTRL_W11: u32 = 11;
pub const BUTTONMATRIX_CTRL_W12: u32 = 12;
pub const BUTTONMATRIX_CTRL_W13: u32 = 13;
pub const BUTTONMATRIX_CTRL_W14: u32 = 14;
pub const BUTTONMATRIX_CTRL_W15: u32 = 15;

pub const BUTTONMATRIX_CTRL_HIDDEN: u32 = 0x0010;
pub const BUTTONMATRIX_CTRL_NO_REPEAT: u32 = 0x0020;
pub const BUTTONMATRIX_CTRL_DISABLED: u32 = 0x0040;
pub const BUTTONMATRIX_CTRL_CHECKABLE: u32 = 0x0080;
pub const BUTTONMATRIX_CTRL_CHECKED: u32 = 0x0100;
pub const BUTTONMATRIX_CTRL_CLICK_TRIG: u32 = 0x0200;
pub const BUTTONMATRIX_CTRL_POPOVER: u32 = 0x0400;
pub const BUTTONMATRIX_CTRL_CUSTOM_1: u32 = 0x4000;
pub const BUTTONMATRIX_CTRL_CUSTOM_2: u32 = 0x8000;

pub struct ButtonMatrix {
    obj: LvObj,
    real_button_count: Cell<Option<usize>>,
}

impl Widget for ButtonMatrix {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl ButtonMatrixMapEntry {
    #[inline]
    pub(crate) const fn as_ptr(self) -> *const c_char {
        self.0
    }
}

fn validate_buttonmatrix_map(map: &ButtonMatrixMap) -> usize {
    let mut real_button_count = 0;

    for entry in map {
        let ptr = entry.as_ptr();
        assert!(!ptr.is_null(), "ButtonMatrix map entries must not be null");

        // SAFETY: ButtonMatrixMapEntry values are constructed from &'static CStr values, so the
        // pointer is valid, NUL-terminated, and static. The null assertion above guards against
        // internal misuse.
        let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
        if bytes.is_empty() {
            return real_button_count;
        }
        if bytes != b"\n" {
            real_button_count += 1;
        }
    }

    panic!("ButtonMatrix map must include an empty string terminator (c\"\")");
}

impl ButtonMatrix {
    /// Creates a new button matrix widget as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer.
    pub fn new(parent: &impl Widget) -> ButtonMatrix {
        let obj = unsafe { c_bindings::lv_buttonmatrix_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_buttonmatrix_create returned null");
        }
        ButtonMatrix {
            obj: LvObj::from_raw(obj),
            real_button_count: Cell::new(None),
        }
    }

    /// Sets the static button text map.
    ///
    /// The map must end with `c""`; use `c"\n"` to start a new row. LVGL keeps
    /// a reference to the map, so it must live for the life of this widget.
    pub fn map(&self, map: &'static ButtonMatrixMap) -> &Self {
        let real_button_count = validate_buttonmatrix_map(map);
        let map_ptr = map.as_ptr() as *const *const c_char;
        // SAFETY: ButtonMatrixMapEntry is #[repr(transparent)] over *const c_char, so the slice
        // can be passed as LVGL's expected pointer array. Entries are constructed from &'static
        // CStr values, and the map slice is &'static because LVGL retains the pointer array. The
        // map was validated to include c"", so LVGL will find its terminator. LVGL reads these
        // string pointers but does not mutate the strings.
        unsafe {
            c_bindings::lv_buttonmatrix_set_map(self.lv_obj().raw(), map_ptr);
        }
        self.real_button_count.set(Some(real_button_count));
        self
    }

    /// Sets the static control map.
    ///
    /// Include one entry per actual button, excluding row separators and the
    /// terminator from the paired [`ButtonMatrixMap`].
    pub fn ctrl_map(&self, ctrl_map: &'static ButtonMatrixCtrlMap) -> &Self {
        let expected = self.real_button_count.get().unwrap_or_else(|| {
            panic!("ButtonMatrix::ctrl_map() requires ButtonMatrix::map() to be called first")
        });
        assert!(
            ctrl_map.len() == expected,
            "ButtonMatrix ctrl_map length mismatch: expected {} entries, got {}",
            expected,
            ctrl_map.len()
        );
        unsafe {
            c_bindings::lv_buttonmatrix_set_ctrl_map(self.lv_obj().raw(), ctrl_map.as_ptr());
        }
        self
    }

    /// Sets one button's relative width in the range 1..=15.
    pub fn button_width(&self, button_id: u32, width: u32) -> &Self {
        assert!(
            (1..=15).contains(&width),
            "ButtonMatrix button width must be in 1..=15, got {}",
            width
        );
        unsafe {
            c_bindings::lv_buttonmatrix_set_button_width(self.lv_obj().raw(), button_id, width);
        }
        self
    }

    /// Sets one or more control flags on a button.
    pub fn set_button_ctrl(&self, button_id: u32, ctrl: u32) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_set_button_ctrl(self.lv_obj().raw(), button_id, ctrl);
        }
        self
    }

    /// Clears one or more control flags from a button.
    pub fn clear_button_ctrl(&self, button_id: u32, ctrl: u32) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_clear_button_ctrl(self.lv_obj().raw(), button_id, ctrl);
        }
        self
    }

    /// Sets one or more control flags on every button.
    pub fn set_button_ctrl_all(&self, ctrl: u32) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_set_button_ctrl_all(self.lv_obj().raw(), ctrl);
        }
        self
    }

    /// Clears one or more control flags from every button.
    pub fn clear_button_ctrl_all(&self, ctrl: u32) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_clear_button_ctrl_all(self.lv_obj().raw(), ctrl);
        }
        self
    }

    /// Enables or disables radio-like behavior where only one checkable button is checked.
    pub fn one_checked(&self, enabled: bool) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_set_one_checked(self.lv_obj().raw(), enabled);
        }
        self
    }

    /// Returns the most recently activated button, or `None` when LVGL reports no selection.
    #[must_use]
    pub fn get_selected_button(&self) -> Option<u32> {
        let selected =
            unsafe { c_bindings::lv_buttonmatrix_get_selected_button(self.lv_obj().raw()) };
        if selected == BUTTONMATRIX_BUTTON_NONE {
            None
        } else {
            Some(selected)
        }
    }

    /// Returns the text for a button id, or `None` if LVGL returns a null pointer.
    #[must_use]
    pub fn get_button_text(&self, button_id: u32) -> Option<&CStr> {
        let ptr =
            unsafe { c_bindings::lv_buttonmatrix_get_button_text(self.lv_obj().raw(), button_id) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::screen::Screen;

    static MAP: &ButtonMatrixMap = &[
        ButtonMatrixMapEntry::new(c"1"),
        ButtonMatrixMapEntry::new(c"2"),
        ButtonMatrixMapEntry::new(c"\n"),
        ButtonMatrixMapEntry::new(c"Action"),
        ButtonMatrixMapEntry::new(c""),
    ];

    static CTRL: &ButtonMatrixCtrlMap = &[
        BUTTONMATRIX_CTRL_W1,
        BUTTONMATRIX_CTRL_CHECKABLE | BUTTONMATRIX_CTRL_W2,
        BUTTONMATRIX_CTRL_DISABLED | BUTTONMATRIX_CTRL_W3,
    ];

    static UNTERMINATED_MAP: &ButtonMatrixMap = &[ButtonMatrixMapEntry::new(c"Only")];

    static CTRL_SHORT: &ButtonMatrixCtrlMap = &[BUTTONMATRIX_CTRL_W1, BUTTONMATRIX_CTRL_W2];

    static CTRL_LONG: &ButtonMatrixCtrlMap = &[
        BUTTONMATRIX_CTRL_W1,
        BUTTONMATRIX_CTRL_W2,
        BUTTONMATRIX_CTRL_W3,
        BUTTONMATRIX_CTRL_W4,
    ];

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_records_create() {
        let p = parent();
        let _ = ButtonMatrix::new(&p);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ButtonMatrixCreate { .. })),
            "expected ButtonMatrixCreate, got: {:?}",
            calls
        );
    }

    #[test]
    fn map_records_labels() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        spy_drain();
        matrix.map(MAP);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetMap { labels, .. }
                    if labels == &vec![b"1\0".to_vec(), b"2\0".to_vec(), b"\n\0".to_vec(), b"Action\0".to_vec(), b"\0".to_vec()]
            )),
            "expected ButtonMatrixSetMap labels, got: {:?}",
            calls
        );
    }

    #[test]
    #[should_panic(expected = "ButtonMatrix map must include an empty string terminator")]
    fn unterminated_map_panics() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        matrix.map(UNTERMINATED_MAP);
    }

    #[test]
    fn valid_map_with_row_separator_counts_only_real_buttons() {
        assert_eq!(validate_buttonmatrix_map(MAP), 3);
    }

    #[test]
    fn ctrl_map_records_values() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        matrix.map(MAP);
        spy_drain();
        matrix.ctrl_map(CTRL);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetCtrlMap { ctrl, .. } if ctrl == &CTRL.to_vec()
            )),
            "expected ButtonMatrixSetCtrlMap, got: {:?}",
            calls
        );
    }

    #[test]
    #[should_panic(expected = "ButtonMatrix::ctrl_map() requires ButtonMatrix::map()")]
    fn ctrl_map_before_map_panics() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        matrix.ctrl_map(CTRL);
    }

    #[test]
    #[should_panic(expected = "ButtonMatrix ctrl_map length mismatch: expected 3 entries, got 2")]
    fn ctrl_map_shorter_than_real_button_count_panics() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        matrix.map(MAP);
        matrix.ctrl_map(CTRL_SHORT);
    }

    #[test]
    #[should_panic(expected = "ButtonMatrix ctrl_map length mismatch: expected 3 entries, got 4")]
    fn ctrl_map_longer_than_real_button_count_panics() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        matrix.map(MAP);
        matrix.ctrl_map(CTRL_LONG);
    }

    #[test]
    fn button_width_records_call() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        spy_drain();
        matrix.button_width(2, 4);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetButtonWidth {
                    btn_id: 2,
                    width: 4,
                    ..
                }
            )),
            "expected ButtonMatrixSetButtonWidth, got: {:?}",
            calls
        );
    }

    #[test]
    #[should_panic(expected = "ButtonMatrix button width must be in 1..=15, got 0")]
    fn invalid_button_width_panics() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        matrix.button_width(2, 0);
    }

    #[test]
    fn ctrl_methods_record_calls() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        spy_drain();
        matrix
            .set_button_ctrl(1, BUTTONMATRIX_CTRL_CHECKABLE | BUTTONMATRIX_CTRL_NO_REPEAT)
            .clear_button_ctrl(1, BUTTONMATRIX_CTRL_NO_REPEAT)
            .set_button_ctrl_all(BUTTONMATRIX_CTRL_CLICK_TRIG)
            .clear_button_ctrl_all(BUTTONMATRIX_CTRL_CLICK_TRIG);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetButtonCtrl { btn_id: 1, ctrl, .. }
                    if *ctrl == (BUTTONMATRIX_CTRL_CHECKABLE | BUTTONMATRIX_CTRL_NO_REPEAT)
            )),
            "expected set button ctrl, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixClearButtonCtrl { btn_id: 1, ctrl, .. }
                    if *ctrl == BUTTONMATRIX_CTRL_NO_REPEAT
            )),
            "expected clear button ctrl, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetButtonCtrlAll { ctrl, .. }
                    if *ctrl == BUTTONMATRIX_CTRL_CLICK_TRIG
            )),
            "expected set all ctrl, got: {:?}",
            calls
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixClearButtonCtrlAll { ctrl, .. }
                    if *ctrl == BUTTONMATRIX_CTRL_CLICK_TRIG
            )),
            "expected clear all ctrl, got: {:?}",
            calls
        );
    }

    #[test]
    fn one_checked_records_call() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        spy_drain();
        matrix.one_checked(true);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ButtonMatrixSetOneChecked { en: true, .. })),
            "expected ButtonMatrixSetOneChecked, got: {:?}",
            calls
        );
    }

    #[test]
    fn selected_button_defaults_to_none() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        assert_eq!(matrix.get_selected_button(), None);
    }

    #[test]
    fn get_button_text_skips_newlines() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        matrix.map(MAP);
        assert_eq!(
            matrix.get_button_text(0).map(CStr::to_bytes),
            Some(&b"1"[..])
        );
        assert_eq!(
            matrix.get_button_text(1).map(CStr::to_bytes),
            Some(&b"2"[..])
        );
        assert_eq!(
            matrix.get_button_text(2).map(CStr::to_bytes),
            Some(&b"Action"[..])
        );
        assert_eq!(matrix.get_button_text(3), None);
    }

    #[test]
    fn chaining_returns_self() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        let result = matrix
            .map(MAP)
            .ctrl_map(CTRL)
            .button_width(2, 2)
            .one_checked(true)
            .set_button_ctrl_all(BUTTONMATRIX_CTRL_CLICK_TRIG)
            .clear_button_ctrl_all(BUTTONMATRIX_CTRL_CLICK_TRIG);
        assert!(core::ptr::eq(result, &matrix));
    }
}
