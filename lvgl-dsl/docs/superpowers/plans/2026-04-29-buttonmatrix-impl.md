# ButtonMatrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an LVGL v9.2 `ButtonMatrix` DSL wrapper with static map/ctrl-map support, tests, reference docs, and playground coverage.

**Architecture:** Add one focused Rust widget module (`src/lvgl/buttonmatrix.rs`) that mirrors the existing small-widget pattern and reuses the existing `Widget` trait. Extend `c_bindings.rs` only where needed for desktop/mock declarations and spy-backed unit tests. Keep docs and playground updates as separate tasks so API behavior is validated before public examples are synced.

**Tech Stack:** Rust 2024, `no_std` crate with `extern crate alloc`, LVGL v9.2 C API, existing mock `c_bindings` spy layer, Markdown reference docs, standalone HTML/CSS/JavaScript playground.

---

## File Structure

- Create `src/lvgl/buttonmatrix.rs`: owns `ButtonMatrix`, static map helper types, ctrl constants, chainable methods, and widget tests.
- Modify `src/lvgl/mod.rs`: add the module and public re-exports.
- Modify `src/lvgl/prelude.rs`: re-export the public button matrix API for `use lvgl_dsl::lvgl::prelude::*`.
- Modify `src/c_bindings.rs`: add desktop declarations, mock spy variants/state/functions, and symbol-reference tests for new LVGL calls.
- Modify `DSL_REFERENCE.md`: add ButtonMatrix to the table of contents, widgets section, shared widget list, and supporting types.
- Modify `DSL_PLAYGROUND.html`: add ButtonMatrix quick link, styles, controls, state, renderer, and generated code.

## Implementation Tasks

### Task 1: Extend ButtonMatrix bindings and mock spy support

**Files:**
- Modify: `src/c_bindings.rs`

- [ ] **Step 1: Write failing binding symbol references**

Add these references to `src/c_bindings.rs` inside `#[cfg(test)] mod tests`, in `task1_new_symbols_referenced()` after the existing `lv_group_focus_obj` reference:

```rust
        let _ = lv_buttonmatrix_set_ctrl_map
            as unsafe fn(*mut lv_obj_t, *const u32);
        let _ = lv_buttonmatrix_set_button_width
            as unsafe fn(*mut lv_obj_t, u32, u32);
        let _ = lv_buttonmatrix_set_button_ctrl
            as unsafe fn(*mut lv_obj_t, u32, u32);
        let _ = lv_buttonmatrix_clear_button_ctrl
            as unsafe fn(*mut lv_obj_t, u32, u32);
        let _ = lv_buttonmatrix_set_button_ctrl_all
            as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_buttonmatrix_clear_button_ctrl_all
            as unsafe fn(*mut lv_obj_t, u32);
        let _ = lv_buttonmatrix_set_one_checked
            as unsafe fn(*mut lv_obj_t, bool);
```

- [ ] **Step 2: Run the targeted binding test to verify it fails**

Run:

```bash
cargo test c_bindings::tests::task1_new_symbols_referenced --quiet
```

Expected: FAIL with unresolved names such as `cannot find value lv_buttonmatrix_set_ctrl_map in this scope`.

- [ ] **Step 3: Add desktop-sim extern declarations**

In `src/c_bindings.rs`, in the `desktop` module after `lv_buttonmatrix_set_map`, add:

```rust
        pub fn lv_buttonmatrix_set_ctrl_map(
            obj: *mut lv_obj_t,
            ctrl_map: *const u32,
        );
        pub fn lv_buttonmatrix_set_button_width(
            obj: *mut lv_obj_t,
            btn_id: u32,
            width: u32,
        );
        pub fn lv_buttonmatrix_set_button_ctrl(
            obj: *mut lv_obj_t,
            btn_id: u32,
            ctrl: u32,
        );
        pub fn lv_buttonmatrix_clear_button_ctrl(
            obj: *mut lv_obj_t,
            btn_id: u32,
            ctrl: u32,
        );
        pub fn lv_buttonmatrix_set_button_ctrl_all(
            obj: *mut lv_obj_t,
            ctrl: u32,
        );
        pub fn lv_buttonmatrix_clear_button_ctrl_all(
            obj: *mut lv_obj_t,
            ctrl: u32,
        );
        pub fn lv_buttonmatrix_set_one_checked(obj: *mut lv_obj_t, en: bool);
```

- [ ] **Step 4: Add mock spy variants**

In the mock `LvCall` enum, replace the existing `ButtonMatrixSetPopovers` line with this group so all buttonmatrix calls sort together:

```rust
        ButtonMatrixCreate        { obj: usize, parent: usize },
        ButtonMatrixSetMap        { obj: usize, labels: Vec<Vec<u8>> },
        ButtonMatrixSetCtrlMap    { obj: usize, ctrl: Vec<u32> },
        ButtonMatrixSetButtonWidth { obj: usize, btn_id: u32, width: u32 },
        ButtonMatrixSetButtonCtrl { obj: usize, btn_id: u32, ctrl: u32 },
        ButtonMatrixClearButtonCtrl { obj: usize, btn_id: u32, ctrl: u32 },
        ButtonMatrixSetButtonCtrlAll { obj: usize, ctrl: u32 },
        ButtonMatrixClearButtonCtrlAll { obj: usize, ctrl: u32 },
        ButtonMatrixSetOneChecked { obj: usize, en: bool },
        ButtonMatrixGetSelectedButton { obj: usize, ret: u32 },
        ButtonMatrixGetButtonText { obj: usize, btn_id: u32, text: Option<Vec<u8>> },
        ButtonMatrixSetPopovers   { obj: usize, en: bool },
```

- [ ] **Step 5: Add mock buttonmatrix state registries**

In the mock `thread_local!` block that contains `DROPDOWN_SELECTED`, add:

```rust
        static BUTTONMATRIX_MAPS:
            RefCell<HashMap<usize, Vec<*const core::ffi::c_char>>> = RefCell::new(HashMap::new());
        static BUTTONMATRIX_CTRLS:
            RefCell<HashMap<usize, Vec<u32>>> = RefCell::new(HashMap::new());
        static BUTTONMATRIX_SELECTED:
            RefCell<HashMap<usize, u32>> = RefCell::new(HashMap::new());
```

In `reset_obj_pool()`, after `DROPDOWN_SELECTED.with(...)`, add:

```rust
        BUTTONMATRIX_MAPS.with(|m| m.borrow_mut().clear());
        BUTTONMATRIX_CTRLS.with(|m| m.borrow_mut().clear());
        BUTTONMATRIX_SELECTED.with(|m| m.borrow_mut().clear());
```

- [ ] **Step 6: Replace the mock ButtonMatrix functions**

Replace the mock `// ButtonMatrix accessors` and `// ButtonMatrix widget (for accent popup)` functions with:

```rust
    const LV_BUTTONMATRIX_BUTTON_NONE: u32 = 0xFFFF;

    unsafe fn read_buttonmatrix_map(
        map: *const *const core::ffi::c_char,
    ) -> (Vec<*const core::ffi::c_char>, Vec<Vec<u8>>) {
        let mut pointers = Vec::new();
        let mut labels = Vec::new();
        let mut index = 0usize;

        loop {
            let ptr = unsafe { *map.add(index) };
            let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes_with_nul().to_vec();
            pointers.push(ptr);
            labels.push(bytes.clone());
            index += 1;
            if bytes == b"\0" {
                break;
            }
        }

        (pointers, labels)
    }

    pub unsafe fn lv_buttonmatrix_get_selected_button(obj: *mut lv_obj_t) -> u32 {
        let ret = BUTTONMATRIX_SELECTED.with(|m| {
            m.borrow().get(&(obj as usize)).copied().unwrap_or(LV_BUTTONMATRIX_BUTTON_NONE)
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixGetSelectedButton {
            obj: obj as usize,
            ret,
        }));
        ret
    }

    pub unsafe fn lv_buttonmatrix_get_button_text(
        obj: *const lv_obj_t,
        btn_id: u32,
    ) -> *const core::ffi::c_char {
        let obj_key = obj as usize;
        let ptr = BUTTONMATRIX_MAPS.with(|maps| {
            let maps = maps.borrow();
            let Some(entries) = maps.get(&obj_key) else {
                return core::ptr::null();
            };

            let mut logical_id = 0u32;
            for entry in entries {
                let bytes = unsafe { CStr::from_ptr(*entry) }.to_bytes();
                if bytes.is_empty() {
                    break;
                }
                if bytes == b"\n" {
                    continue;
                }
                if logical_id == btn_id {
                    return *entry;
                }
                logical_id += 1;
            }

            core::ptr::null()
        });

        let text = if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr) }.to_bytes_with_nul().to_vec())
        };
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixGetButtonText {
            obj: obj_key,
            btn_id,
            text,
        }));
        ptr
    }

    pub unsafe fn lv_buttonmatrix_create(parent: *mut lv_obj_t) -> *mut lv_obj_t {
        let obj = alloc_fake_obj();
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixCreate {
            obj: obj as usize,
            parent: parent as usize,
        }));
        obj
    }

    pub unsafe fn lv_buttonmatrix_set_map(
        obj: *mut lv_obj_t,
        map: *const *const core::ffi::c_char,
    ) {
        let (pointers, labels) = unsafe { read_buttonmatrix_map(map) };
        BUTTONMATRIX_MAPS.with(|m| {
            m.borrow_mut().insert(obj as usize, pointers);
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixSetMap {
            obj: obj as usize,
            labels,
        }));
    }

    pub unsafe fn lv_buttonmatrix_set_ctrl_map(
        obj: *mut lv_obj_t,
        ctrl_map: *const u32,
    ) {
        let button_count = BUTTONMATRIX_MAPS.with(|maps| {
            maps.borrow()
                .get(&(obj as usize))
                .map(|entries| {
                    entries.iter().filter(|entry| {
                        let bytes = unsafe { CStr::from_ptr(**entry) }.to_bytes();
                        !bytes.is_empty() && bytes != b"\n"
                    }).count()
                })
                .unwrap_or(0)
        });

        let ctrl = unsafe { core::slice::from_raw_parts(ctrl_map, button_count) }.to_vec();
        BUTTONMATRIX_CTRLS.with(|m| {
            m.borrow_mut().insert(obj as usize, ctrl.clone());
        });
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixSetCtrlMap {
            obj: obj as usize,
            ctrl,
        }));
    }

    pub unsafe fn lv_buttonmatrix_set_button_width(
        obj: *mut lv_obj_t,
        btn_id: u32,
        width: u32,
    ) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixSetButtonWidth {
            obj: obj as usize,
            btn_id,
            width,
        }));
    }

    pub unsafe fn lv_buttonmatrix_set_button_ctrl(
        obj: *mut lv_obj_t,
        btn_id: u32,
        ctrl: u32,
    ) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixSetButtonCtrl {
            obj: obj as usize,
            btn_id,
            ctrl,
        }));
    }

    pub unsafe fn lv_buttonmatrix_clear_button_ctrl(
        obj: *mut lv_obj_t,
        btn_id: u32,
        ctrl: u32,
    ) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixClearButtonCtrl {
            obj: obj as usize,
            btn_id,
            ctrl,
        }));
    }

    pub unsafe fn lv_buttonmatrix_set_button_ctrl_all(obj: *mut lv_obj_t, ctrl: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixSetButtonCtrlAll {
            obj: obj as usize,
            ctrl,
        }));
    }

    pub unsafe fn lv_buttonmatrix_clear_button_ctrl_all(obj: *mut lv_obj_t, ctrl: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixClearButtonCtrlAll {
            obj: obj as usize,
            ctrl,
        }));
    }

    pub unsafe fn lv_buttonmatrix_set_one_checked(obj: *mut lv_obj_t, en: bool) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ButtonMatrixSetOneChecked {
            obj: obj as usize,
            en,
        }));
    }
```

- [ ] **Step 7: Run the targeted binding test**

Run:

```bash
cargo test c_bindings::tests::task1_new_symbols_referenced --quiet
```

Expected: PASS.

- [ ] **Step 8: Commit bindings**

Run:

```bash
git add src/c_bindings.rs
git commit -m "test: add buttonmatrix binding spies" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 2: Add the ButtonMatrix wrapper with TDD

**Files:**
- Create: `src/lvgl/buttonmatrix.rs`
- Modify: `src/lvgl/mod.rs`
- Modify: `src/lvgl/prelude.rs`

- [ ] **Step 1: Create failing wrapper tests**

Create `src/lvgl/buttonmatrix.rs` with this test-first skeleton:

```rust
use core::ffi::{c_char, CStr};

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

unsafe impl Sync for ButtonMatrixMapEntry {}
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
}

impl Widget for ButtonMatrix {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{reset_obj_pool, spy_drain, LvCall};
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
            calls.iter().any(|c| matches!(c, LvCall::ButtonMatrixCreate { .. })),
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
    fn button_width_records_call() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        spy_drain();
        matrix.button_width(2, 4);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetButtonWidth { btn_id: 2, width: 4, .. }
            )),
            "expected ButtonMatrixSetButtonWidth, got: {:?}",
            calls
        );
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
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::ButtonMatrixSetButtonCtrl { btn_id: 1, ctrl, .. }
                if *ctrl == (BUTTONMATRIX_CTRL_CHECKABLE | BUTTONMATRIX_CTRL_NO_REPEAT)
        )), "expected set button ctrl, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::ButtonMatrixClearButtonCtrl { btn_id: 1, ctrl, .. }
                if *ctrl == BUTTONMATRIX_CTRL_NO_REPEAT
        )), "expected clear button ctrl, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::ButtonMatrixSetButtonCtrlAll { ctrl, .. }
                if *ctrl == BUTTONMATRIX_CTRL_CLICK_TRIG
        )), "expected set all ctrl, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::ButtonMatrixClearButtonCtrlAll { ctrl, .. }
                if *ctrl == BUTTONMATRIX_CTRL_CLICK_TRIG
        )), "expected clear all ctrl, got: {:?}", calls);
    }

    #[test]
    fn one_checked_records_call() {
        let p = parent();
        let matrix = ButtonMatrix::new(&p);
        spy_drain();
        matrix.one_checked(true);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(c, LvCall::ButtonMatrixSetOneChecked { en: true, .. })),
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
        assert_eq!(matrix.get_button_text(0).map(CStr::to_bytes), Some(&b"1"[..]));
        assert_eq!(matrix.get_button_text(1).map(CStr::to_bytes), Some(&b"2"[..]));
        assert_eq!(matrix.get_button_text(2).map(CStr::to_bytes), Some(&b"Action"[..]));
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
```

- [ ] **Step 2: Wire the module enough to run tests and verify failure**

In `src/lvgl/mod.rs`, add:

```rust
mod buttonmatrix;
```

near the other widget modules, and add this public re-export near `Button`:

```rust
pub use self::buttonmatrix::{
    ButtonMatrix, ButtonMatrixCtrlMap, ButtonMatrixMap, ButtonMatrixMapEntry,
    BUTTONMATRIX_BUTTON_NONE, BUTTONMATRIX_CTRL_CHECKABLE, BUTTONMATRIX_CTRL_CHECKED,
    BUTTONMATRIX_CTRL_CLICK_TRIG, BUTTONMATRIX_CTRL_CUSTOM_1, BUTTONMATRIX_CTRL_CUSTOM_2,
    BUTTONMATRIX_CTRL_DISABLED, BUTTONMATRIX_CTRL_HIDDEN, BUTTONMATRIX_CTRL_NO_REPEAT,
    BUTTONMATRIX_CTRL_POPOVER, BUTTONMATRIX_CTRL_W1, BUTTONMATRIX_CTRL_W10,
    BUTTONMATRIX_CTRL_W11, BUTTONMATRIX_CTRL_W12, BUTTONMATRIX_CTRL_W13,
    BUTTONMATRIX_CTRL_W14, BUTTONMATRIX_CTRL_W15, BUTTONMATRIX_CTRL_W2,
    BUTTONMATRIX_CTRL_W3, BUTTONMATRIX_CTRL_W4, BUTTONMATRIX_CTRL_W5,
    BUTTONMATRIX_CTRL_W6, BUTTONMATRIX_CTRL_W7, BUTTONMATRIX_CTRL_W8,
    BUTTONMATRIX_CTRL_W9,
};
```

In `src/lvgl/prelude.rs`, add the matching `pub use super::buttonmatrix::{...};` line with the same item list.

Run:

```bash
cargo test lvgl::buttonmatrix --quiet
```

Expected: FAIL with missing associated functions and methods such as `ButtonMatrix::new`, `map`, and `ctrl_map`.

- [ ] **Step 3: Implement ButtonMatrix methods**

In `src/lvgl/buttonmatrix.rs`, add this inherent impl before the test module:

```rust
impl ButtonMatrixMapEntry {
    #[inline]
    pub(crate) const fn as_ptr(self) -> *const c_char {
        self.0
    }
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
        ButtonMatrix { obj: LvObj::from_raw(obj) }
    }

    /// Sets the static button text map.
    ///
    /// The map must end with `c""`; use `c"\n"` to start a new row. LVGL keeps
    /// a reference to the map, so it must live for the life of this widget.
    pub fn map(&self, map: &'static ButtonMatrixMap) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_set_map(
                self.lv_obj().raw(),
                map.as_ptr() as *const *const c_char,
            );
        }
        self
    }

    /// Sets the static control map.
    ///
    /// Include one entry per actual button, excluding row separators and the
    /// terminator from the paired [`ButtonMatrixMap`].
    pub fn ctrl_map(&self, ctrl_map: &'static ButtonMatrixCtrlMap) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_set_ctrl_map(
                self.lv_obj().raw(),
                ctrl_map.as_ptr(),
            );
        }
        self
    }

    /// Sets one button's relative width in the range 1..=15.
    pub fn button_width(&self, button_id: u32, width: u32) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_set_button_width(
                self.lv_obj().raw(),
                button_id,
                width,
            );
        }
        self
    }

    /// Sets one or more control flags on a button.
    pub fn set_button_ctrl(&self, button_id: u32, ctrl: u32) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_set_button_ctrl(
                self.lv_obj().raw(),
                button_id,
                ctrl,
            );
        }
        self
    }

    /// Clears one or more control flags from a button.
    pub fn clear_button_ctrl(&self, button_id: u32, ctrl: u32) -> &Self {
        unsafe {
            c_bindings::lv_buttonmatrix_clear_button_ctrl(
                self.lv_obj().raw(),
                button_id,
                ctrl,
            );
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
        let selected = unsafe { c_bindings::lv_buttonmatrix_get_selected_button(self.lv_obj().raw()) };
        if selected == BUTTONMATRIX_BUTTON_NONE {
            None
        } else {
            Some(selected)
        }
    }

    /// Returns the text for a button id, or `None` if LVGL returns a null pointer.
    #[must_use]
    pub fn get_button_text(&self, button_id: u32) -> Option<&CStr> {
        let ptr = unsafe {
            c_bindings::lv_buttonmatrix_get_button_text(self.lv_obj().raw(), button_id)
        };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr) })
        }
    }
}
```

- [ ] **Step 4: Run wrapper tests**

Run:

```bash
cargo test lvgl::buttonmatrix --quiet
```

Expected: PASS.

- [ ] **Step 5: Run formatting**

Run:

```bash
cargo fmt
```

Expected: command exits successfully and formats touched Rust files.

- [ ] **Step 6: Commit wrapper**

Run:

```bash
git add src/lvgl/buttonmatrix.rs src/lvgl/mod.rs src/lvgl/prelude.rs
git commit -m "feat: add buttonmatrix wrapper" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Update DSL reference documentation

**Files:**
- Modify: `DSL_REFERENCE.md`

- [ ] **Step 1: Add ButtonMatrix to the contents**

In `DSL_REFERENCE.md`, add `ButtonMatrix` after `Button` in the widgets table of contents:

```markdown
    - [ButtonMatrix](#buttonmatrix)
```

Add supporting type entries after `ImageButtonState`:

```markdown
    - [ButtonMatrixMapEntry / ButtonMatrixMap](#buttonmatrixmapentry--buttonmatrixmap)
    - [ButtonMatrixCtrlMap](#buttonmatrixctrlmap)
```

- [ ] **Step 2: Add the ButtonMatrix widget section**

Insert this section after the Button section and before Label:

````markdown
### ButtonMatrix

A lightweight LVGL button matrix widget (`lv_buttonmatrix_create`) that displays many virtual buttons from a static text map. Buttons are not separate child widgets, so this is much cheaper than building rows of `Button` + `Label` objects.

Requires `CONFIG_LV_USE_BUTTONMATRIX=y` in the LVGL/Kconfig configuration.

**Construction**

```rust
let matrix = ButtonMatrix::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `map(&'static ButtonMatrixMap)` | Sets the static text map. The map must end with `c""`; use `c"\n"` for row breaks. LVGL keeps a pointer to this array. |
| `ctrl_map(&'static ButtonMatrixCtrlMap)` | Sets width/control values for each actual button, excluding row breaks and the terminator. |
| `button_width(button_id, width)` | Sets one button's relative width (`1..=15`) inside its row. Prefer `ctrl_map` for initial layout. |
| `set_button_ctrl(button_id, ctrl)` | Sets one or more `BUTTONMATRIX_CTRL_*` flags on a button. |
| `clear_button_ctrl(button_id, ctrl)` | Clears one or more flags from a button. |
| `set_button_ctrl_all(ctrl)` | Sets one or more flags on every button. |
| `clear_button_ctrl_all(ctrl)` | Clears one or more flags from every button. |
| `one_checked(bool)` | Enables radio-button-like behavior where only one checkable button can be checked at a time. |
| `get_selected_button() -> Option<u32>` | Returns the most recently activated button, or `None` if LVGL has no selection. |
| `get_button_text(button_id) -> Option<&CStr>` | Returns a button's text, or `None` if LVGL returns a null pointer. |

**Example**

```rust
use lvgl_dsl::lvgl::prelude::*;

static NUMPAD_MAP: &ButtonMatrixMap = &[
    ButtonMatrixMapEntry::new(c"1"),
    ButtonMatrixMapEntry::new(c"2"),
    ButtonMatrixMapEntry::new(c"3"),
    ButtonMatrixMapEntry::new(c"\n"),
    ButtonMatrixMapEntry::new(c"4"),
    ButtonMatrixMapEntry::new(c"5"),
    ButtonMatrixMapEntry::new(c"6"),
    ButtonMatrixMapEntry::new(c"\n"),
    ButtonMatrixMapEntry::new(c"Action"),
    ButtonMatrixMapEntry::new(c"Cancel"),
    ButtonMatrixMapEntry::new(c""),
];

static NUMPAD_CTRL: &ButtonMatrixCtrlMap = &[
    BUTTONMATRIX_CTRL_W1,
    BUTTONMATRIX_CTRL_W1,
    BUTTONMATRIX_CTRL_W1,
    BUTTONMATRIX_CTRL_W1,
    BUTTONMATRIX_CTRL_CHECKABLE | BUTTONMATRIX_CTRL_W1,
    BUTTONMATRIX_CTRL_W1,
    BUTTONMATRIX_CTRL_CHECKABLE | BUTTONMATRIX_CTRL_W2,
    BUTTONMATRIX_CTRL_DISABLED | BUTTONMATRIX_CTRL_W1,
];

let matrix = ButtonMatrix::new(&container)
    .map(NUMPAD_MAP)
    .ctrl_map(NUMPAD_CTRL)
    .one_checked(true)
    .align(LvAlign::Center, 0, 0)
    .on_event(|_| { /* read selection in app code */ }, LvEventCode::ValueChanged);
```

> **Lifetime rule:** `ButtonMatrixMap` must be static because LVGL keeps a pointer to the map. `ButtonMatrixCtrlMap` is also accepted as static by this DSL for consistency with the map API.

````

- [ ] **Step 3: Update the shared widget list**

At the line that lists every widget implementing `Widget`, change:

```markdown
Every widget (`Screen`, `Obj`, `Button`, `Label`, `Dropdown`, `QrCode`, `Image`, `ImageButton`, `TextArea`, `Keyboard`) implements the `Widget` trait.
```

to:

```markdown
Every widget (`Screen`, `Obj`, `Button`, `ButtonMatrix`, `Label`, `Dropdown`, `QrCode`, `Image`, `ImageButton`, `TextArea`, `Keyboard`) implements the `Widget` trait.
```

- [ ] **Step 4: Add supporting type sections**

Insert these sections after `ImageButtonState`:

````markdown
### ButtonMatrixMapEntry / ButtonMatrixMap

`ButtonMatrixMapEntry` wraps a static C string pointer for LVGL button matrix maps. Use `ButtonMatrixMapEntry::new(c"...")` with Rust 2024 C string literals. `ButtonMatrixMap` is a slice of entries.

```rust
static MAP: &ButtonMatrixMap = &[
    ButtonMatrixMapEntry::new(c"Yes"),
    ButtonMatrixMapEntry::new(c"No"),
    ButtonMatrixMapEntry::new(c"\n"),
    ButtonMatrixMapEntry::new(c"Cancel"),
    ButtonMatrixMapEntry::new(c""),
];
```

The final `c""` terminator is required. `c"\n"` starts a new row and does not count as a button id.

---

### ButtonMatrixCtrlMap

`ButtonMatrixCtrlMap` is a parallel `u32` slice containing one width/control value per actual button. Combine width constants and flags with `|`.

| Constant | Value | Description |
|----------|-------|-------------|
| `BUTTONMATRIX_CTRL_W1` ... `BUTTONMATRIX_CTRL_W15` | `1` ... `15` | Relative button width inside a row. |
| `BUTTONMATRIX_CTRL_HIDDEN` | `0x0010` | Button is invisible and not clickable, but still takes layout space. |
| `BUTTONMATRIX_CTRL_NO_REPEAT` | `0x0020` | Long press does not repeat. |
| `BUTTONMATRIX_CTRL_DISABLED` | `0x0040` | Button is disabled. |
| `BUTTONMATRIX_CTRL_CHECKABLE` | `0x0080` | Button can toggle checked state. |
| `BUTTONMATRIX_CTRL_CHECKED` | `0x0100` | Button starts checked. |
| `BUTTONMATRIX_CTRL_CLICK_TRIG` | `0x0200` | Send value-changed on click instead of press. |
| `BUTTONMATRIX_CTRL_POPOVER` | `0x0400` | Show the button label in a popover while pressed. |
| `BUTTONMATRIX_CTRL_CUSTOM_1` | `0x4000` | Free custom flag. |
| `BUTTONMATRIX_CTRL_CUSTOM_2` | `0x8000` | Free custom flag. |

```rust
static CTRL: &ButtonMatrixCtrlMap = &[
    BUTTONMATRIX_CTRL_W1,
    BUTTONMATRIX_CTRL_CHECKABLE | BUTTONMATRIX_CTRL_W2,
    BUTTONMATRIX_CTRL_DISABLED | BUTTONMATRIX_CTRL_W1,
];
```

````

- [ ] **Step 5: Verify docs mention the new widget**

Run:

```bash
grep -n "ButtonMatrix" DSL_REFERENCE.md | head
```

Expected: output includes lines from the table of contents, widget section, and supporting type sections.

- [ ] **Step 6: Commit docs**

Run:

```bash
git add DSL_REFERENCE.md
git commit -m "docs: add buttonmatrix reference" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 4: Add ButtonMatrix to the playground

**Files:**
- Modify: `DSL_PLAYGROUND.html`

- [ ] **Step 1: Add quick link and section markup**

Add this quick link after Button:

```html
      <a href="#buttonmatrix-playground">ButtonMatrix</a>
```

Insert this playground section after the Button section and before Label:

```html
      <section class="playground" id="buttonmatrix-playground">
        <div class="playground-header">
          <div>
            <h2>2. ButtonMatrix</h2>
            <p>Approximate LVGL's virtual button grid with static map and control-map code generation.</p>
          </div>
          <span class="section-tag">Widget</span>
        </div>
        <div class="playground-body">
          <div class="pane">
            <div class="pane-heading">
              <h3>Live preview</h3>
              <p>Rows come from the map; action flags come from the control map.</p>
            </div>
            <div class="preview-frame">
              <div class="preview-caption">Left preview</div>
              <div class="preview-content" data-preview="buttonmatrix"></div>
            </div>
          </div>
          <div class="pane">
            <div class="pane-heading">
              <h3>Controls</h3>
              <p>Maps to <strong>ButtonMatrix</strong>, <strong>ButtonMatrixMap</strong>, and <strong>ButtonMatrixCtrlMap</strong></p>
            </div>
            <div class="controls-grid">
              <label class="control full">
                <span class="label-row"><span>Rows (one row per line, buttons separated by commas)</span></span>
                <textarea rows="4" style="resize:vertical;width:100%;background:var(--input-bg);color:var(--text);border:1px solid var(--input-stroke);border-radius:8px;padding:8px;font-family:var(--font-code);font-size:13px;" data-section="buttonmatrix" data-key="rowsText" data-type="string">1,2,3
4,5,6
Action,Cancel</textarea>
              </label>
              <label class="control">
                <span class="label-row"><span>Selected button</span><span class="readout" id="buttonmatrix-selected-readout">0</span></span>
                <input type="range" min="0" max="7" step="1" value="0" id="buttonmatrix-selected" data-section="buttonmatrix" data-key="selectedIndex" data-type="number">
              </label>
              <label class="control">
                <span class="label-row"><span>Action width</span><span class="readout" id="buttonmatrix-action-width-readout">2</span></span>
                <input type="range" min="1" max="15" step="1" value="2" data-section="buttonmatrix" data-key="actionWidth" data-type="number">
              </label>
              <label class="control">
                <span class="label-row"><span>Cancel width</span><span class="readout" id="buttonmatrix-cancel-width-readout">1</span></span>
                <input type="range" min="1" max="15" step="1" value="1" data-section="buttonmatrix" data-key="cancelWidth" data-type="number">
              </label>
              <label class="control">
                <span class="label-row"><span>Background colour</span></span>
                <input type="color" value="#1e1e2e" data-section="buttonmatrix" data-key="bgColor" data-type="string">
              </label>
              <label class="control">
                <span class="label-row"><span>Button colour</span></span>
                <input type="color" value="#313244" data-section="buttonmatrix" data-key="buttonColor" data-type="string">
              </label>
              <label class="control">
                <span class="label-row"><span>Checked colour</span></span>
                <input type="color" value="#89b4fa" data-section="buttonmatrix" data-key="checkedColor" data-type="string">
              </label>
              <label class="control">
                <span class="label-row"><span>Text colour</span></span>
                <input type="color" value="#cdd6f4" data-section="buttonmatrix" data-key="textColor" data-type="string">
              </label>
              <div class="control full">
                <span class="label-row"><span>Control flags</span></span>
                <div class="toggle-row">
                  <label class="toggle-pill"><input type="checkbox" checked data-section="buttonmatrix" data-key="oneChecked" data-type="boolean"> one_checked</label>
                  <label class="toggle-pill"><input type="checkbox" checked data-section="buttonmatrix" data-key="checkableActions" data-type="boolean"> checkable actions</label>
                  <label class="toggle-pill"><input type="checkbox" data-section="buttonmatrix" data-key="disableCancel" data-type="boolean"> disable Cancel</label>
                  <label class="toggle-pill"><input type="checkbox" checked data-section="buttonmatrix" data-key="clickTrig" data-type="boolean"> click trigger</label>
                  <label class="toggle-pill"><input type="checkbox" data-section="buttonmatrix" data-key="noRepeat" data-type="boolean"> no repeat</label>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="code-wrap">
          <div class="code-title"><span>Generated DSL</span><span>Static map + control map</span></div>
          <pre><code data-code="buttonmatrix"></code></pre>
        </div>
      </section>
```

- [ ] **Step 2: Add CSS for the preview**

Insert after the dropdown CSS block:

```css
    .buttonmatrix-shell {
      width: min(360px, 100%);
      padding: 14px;
      border-radius: 18px;
      display: grid;
      gap: 10px;
      box-shadow: inset 0 0 0 1px rgba(255,255,255,0.08);
    }
    .buttonmatrix-row {
      display: flex;
      gap: 10px;
    }
    .buttonmatrix-key {
      min-width: 0;
      height: 44px;
      border: 1px solid rgba(255,255,255,0.12);
      border-radius: 10px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      font-weight: 700;
      transition: transform 140ms ease, filter 140ms ease;
    }
    .buttonmatrix-key:not(.is-disabled):hover {
      transform: translateY(-1px);
      filter: brightness(1.12);
    }
    .buttonmatrix-key.is-disabled {
      opacity: 0.42;
      filter: grayscale(0.35);
    }
```

- [ ] **Step 3: Add playground state**

Add this object to the top-level `state` object after `button`:

```js
      buttonmatrix: {
        rowsText: "1,2,3\n4,5,6\nAction,Cancel",
        selectedIndex: 0,
        actionWidth: 2,
        cancelWidth: 1,
        bgColor: "#1e1e2e",
        buttonColor: "#313244",
        checkedColor: "#89b4fa",
        textColor: "#cdd6f4",
        oneChecked: true,
        checkableActions: true,
        disableCancel: false,
        clickTrig: true,
        noRepeat: false
      },
```

- [ ] **Step 4: Add renderer helpers and renderButtonMatrix**

Insert this JavaScript after `renderButton()`:

```js
    function parseButtonMatrixRows(text) {
      return text.split("\n")
        .map(row => row.split(",").map(cell => cell.trim()).filter(Boolean))
        .filter(row => row.length > 0);
    }

    function buttonMatrixEntries(rows) {
      const entries = [];
      rows.forEach((row, rowIndex) => {
        row.forEach(label => entries.push({ label, rowIndex }));
        if (rowIndex < rows.length - 1) entries.push({ label: "\\n", rowIndex });
      });
      return entries;
    }

    function renderButtonMatrix() {
      const s = state.buttonmatrix;
      const rows = parseButtonMatrixRows(s.rowsText);
      const buttons = rows.flat();
      const selected = Math.min(s.selectedIndex, Math.max(0, buttons.length - 1));
      const actionIndex = buttons.findIndex(label => label.toLowerCase() === "action");
      const cancelIndex = buttons.findIndex(label => label.toLowerCase() === "cancel");

      const selectedInput = byId("buttonmatrix-selected");
      if (selectedInput) selectedInput.max = String(Math.max(0, buttons.length - 1));
      setReadout("buttonmatrix-selected-readout", String(selected));
      setReadout("buttonmatrix-action-width-readout", String(s.actionWidth));
      setReadout("buttonmatrix-cancel-width-readout", String(s.cancelWidth));

      const rowsHtml = rows.map((row, rowIndex) => {
        const rowStart = rows.slice(0, rowIndex).reduce((sum, r) => sum + r.length, 0);
        const rowHtml = row.map((label, colIndex) => {
          const index = rowStart + colIndex;
          const lower = label.toLowerCase();
          const isAction = lower === "action";
          const isCancel = lower === "cancel";
          const grow = isAction ? s.actionWidth : isCancel ? s.cancelWidth : 1;
          const checked = s.oneChecked && s.checkableActions && index === selected && (isAction || isCancel);
          const disabled = s.disableCancel && isCancel;
          const bg = checked ? s.checkedColor : s.buttonColor;
          return `<div class="buttonmatrix-key${disabled ? " is-disabled" : ""}" style="flex:${grow} 1 0;background:${bg};color:${s.textColor};">${escapeHtml(label)}</div>`;
        }).join("");
        return `<div class="buttonmatrix-row">${rowHtml}</div>`;
      }).join("");

      const preview = document.querySelector('[data-preview="buttonmatrix"]');
      preview.innerHTML = `<div class="buttonmatrix-shell" style="background:${s.bgColor};">${rowsHtml}</div>`;

      const mapLines = [];
      buttonMatrixEntries(rows).forEach(entry => {
        const value = entry.label === "\\n" ? "\\\\n" : escapeRustString(entry.label);
        mapLines.push(`    ButtonMatrixMapEntry::new(c"${value}"),`);
      });
      mapLines.push(`    ButtonMatrixMapEntry::new(c""),`);

      const ctrlLines = buttons.map((label, index) => {
        const lower = label.toLowerCase();
        const width = lower === "action" ? s.actionWidth : lower === "cancel" ? s.cancelWidth : 1;
        const parts = [`BUTTONMATRIX_CTRL_W${width}`];
        if (s.checkableActions && (lower === "action" || lower === "cancel")) parts.unshift("BUTTONMATRIX_CTRL_CHECKABLE");
        if (s.disableCancel && lower === "cancel") parts.unshift("BUTTONMATRIX_CTRL_DISABLED");
        if (s.clickTrig) parts.unshift("BUTTONMATRIX_CTRL_CLICK_TRIG");
        if (s.noRepeat) parts.unshift("BUTTONMATRIX_CTRL_NO_REPEAT");
        if (s.oneChecked && index === selected && s.checkableActions && (lower === "action" || lower === "cancel")) parts.unshift("BUTTONMATRIX_CTRL_CHECKED");
        return `    ${parts.join(" | ")},`;
      });

      const lines = [
        "static BUTTONS: &ButtonMatrixMap = &[",
        ...mapLines,
        "];",
        "",
        "static BUTTON_CTRLS: &ButtonMatrixCtrlMap = &[",
        ...ctrlLines,
        "];",
        "",
        "let matrix = ButtonMatrix::new(&container)",
        "    .map(BUTTONS)",
        "    .ctrl_map(BUTTON_CTRLS)",
        `    .one_checked(${s.oneChecked})`,
        `    .bg_color(${colorToDsl(s.bgColor)})`,
        `    .text_color(${colorToDsl(s.textColor)})`,
        "    .on_event(|_| { /* value changed */ }, LvEventCode::ValueChanged);"
      ];
      setCode("buttonmatrix", lines.join("\n"));
    }
```

- [ ] **Step 5: Add ButtonMatrix to renderAll**

In `renderAll()`, add:

```js
      renderButtonMatrix();
```

immediately after `renderButton();`.

- [ ] **Step 6: Verify generated code appears in the HTML**

Run:

```bash
grep -n "renderButtonMatrix\\|ButtonMatrixMap\\|buttonmatrix-playground" DSL_PLAYGROUND.html
```

Expected: output includes the new section id, generated code references, and `renderButtonMatrix`.

- [ ] **Step 7: Commit playground**

Run:

```bash
git add DSL_PLAYGROUND.html
git commit -m "docs: add buttonmatrix playground" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 5: Final verification

**Files:**
- Verify: `src/c_bindings.rs`
- Verify: `src/lvgl/buttonmatrix.rs`
- Verify: `src/lvgl/mod.rs`
- Verify: `src/lvgl/prelude.rs`
- Verify: `DSL_REFERENCE.md`
- Verify: `DSL_PLAYGROUND.html`

- [ ] **Step 1: Run the full Rust test suite**

Run:

```bash
cargo test --quiet
```

Expected: PASS.

- [ ] **Step 2: Run formatting check**

Run:

```bash
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 3: Check public docs and playground references**

Run:

```bash
grep -n "ButtonMatrix" DSL_REFERENCE.md | head -20
grep -n "buttonmatrix" DSL_PLAYGROUND.html | head -20
```

Expected: both commands print references to the new widget.

- [ ] **Step 4: Check final git status**

Run:

```bash
git --no-pager status --short
```

Expected: clean working tree after all task commits, or only unrelated user-owned changes.
