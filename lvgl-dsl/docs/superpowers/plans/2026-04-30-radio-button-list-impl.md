# RadioButtonList Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a reusable LVGL `RadioButtonList` composite widget with runtime labels, fixed-height selectable rows, configurable styles, enabled state, and typed change callbacks.

**Architecture:** Implement the widget as a composite root object containing one row object per option, with each row containing an indicator object and a label. Store labels, enabled flags, selected state, row widget pointers, style values, and callback closure in Rust-owned widget state; row click trampolines recover the owning list and row index from stable boxed contexts. Keep mock/spy additions separate from widget behavior so tests can verify styling and layout precisely.

**Tech Stack:** Rust 2024, `no_std` + `alloc`, LVGL v9 C bindings, existing `Widget` trait, desktop/mock LVGL spy tests, `cargo test`.

---

## File Structure

- Modify: `src/lvgl/color.rs`
  - Make `Color` copyable so style structs can be copied and reapplied across selected/unselected transitions.
- Modify: `src/c_bindings.rs`
  - Add mock `LvCall` variants and recording for style, padding, width, and height setters used by the widget tests.
- Create: `src/lvgl/radiobuttonlist/types.rs`
  - Define `RadioButtonEvent`, style structs, `RadioButtonListConfig`, and reusable validation helpers.
- Create: `src/lvgl/radiobuttonlist/tree.rs`
  - Build the LVGL object tree: root, rows, indicators, and labels.
- Create: `src/lvgl/radiobuttonlist/style.rs`
  - Apply base/selected/disabled styles to rows, indicators, and labels.
- Create: `src/lvgl/radiobuttonlist/trampolines.rs`
  - Register/unregister row click callbacks and recover list state from stable row contexts.
- Create: `src/lvgl/radiobuttonlist/mod.rs`
  - Public `RadioButtonList` type, `Widget` implementation, selection/enabled APIs, callback API, and tests.
- Modify: `src/lvgl/mod.rs`
  - Register the new module and re-export public types.
- Modify: `src/lvgl/prelude.rs`
  - Re-export the new widget and supporting public types.
- Modify: `DSL_REFERENCE.md`
  - Document widget purpose, construction, selection, disabled state, and style API.
- Modify: `DSL_PLAYGROUND.html`
  - Add a RadioButtonList playground section and generated code.

## Task 1: Mock Spy and Color Foundations

**Files:**
- Modify: `src/lvgl/color.rs`
- Modify: `src/c_bindings.rs`

- [ ] **Step 1: Write failing tests for copyable colors and mock style recording**

Add these tests to `src/lvgl/color.rs` inside the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn color_is_copyable_for_style_structs() {
    let a = Color::hex(0xFF6600);
    let b = a;
    let _c = a;
    assert_eq!(b.to_lv(), Color::hex(0xFF6600).to_lv());
}
```

Add this test to `src/c_bindings.rs` inside the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn mock_records_radio_list_style_primitives() {
    use crate::c_bindings::*;

    reset_obj_pool();
    let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
    let color = unsafe { lv_color_hex(0xFF6600) };

    unsafe {
        lv_obj_set_width(obj, 123);
        lv_obj_set_height(obj, 45);
        lv_obj_set_style_pad_left(obj, 11, 0);
        lv_obj_set_style_pad_right(obj, 12, 0);
        lv_obj_set_style_pad_top(obj, 13, 0);
        lv_obj_set_style_pad_bottom(obj, 14, 0);
        lv_obj_set_style_bg_color(obj, color, 0);
        lv_obj_set_style_bg_opa(obj, 200, 0);
        lv_obj_set_style_text_color(obj, color, 0);
        lv_obj_set_style_text_opa(obj, 201, 0);
        lv_obj_set_style_border_color(obj, color, 0);
        lv_obj_set_style_border_width(obj, 2, 0);
        lv_obj_set_style_border_opa(obj, 202, 0);
    }

    let calls = spy_drain();
    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetWidth { value: 123, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetHeight { value: 45, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadLeft { value: 11, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadRight { value: 12, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadTop { value: 13, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadBottom { value: 14, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBgColor { color: c0, .. } if *c0 == color)), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBgOpa { opa: 200, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleTextColor { color: c0, .. } if *c0 == color)), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleTextOpa { opa: 201, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBorderColor { color: c0, .. } if *c0 == color)), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBorderWidth { value: 2, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBorderOpa { opa: 202, .. })), "{calls:?}");
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run:

```bash
cargo test color_is_copyable_for_style_structs mock_records_radio_list_style_primitives --quiet
```

Expected: `color_is_copyable_for_style_structs` fails to compile because `Color` is not `Copy`, and the binding test fails to compile because the new `LvCall` variants do not exist.

- [ ] **Step 3: Make `Color` copyable**

Change the struct definition in `src/lvgl/color.rs` to:

```rust
#[derive(Copy, Clone)]
pub struct Color {
    inner: c_bindings::lv_color_t,
}
```

- [ ] **Step 4: Add mock `LvCall` variants**

In `src/c_bindings.rs`, extend the mock `LvCall` enum with these variants near the existing style and sizing variants:

```rust
ObjSetWidth      { obj: usize, value: i32 },
ObjSetHeight     { obj: usize, value: i32 },
SetStyleBgColor  { obj: usize, color: lv_color_t },
SetStyleBgOpa    { obj: usize, opa: u8 },
SetStyleTextColor { obj: usize, color: lv_color_t },
SetStyleTextOpa  { obj: usize, opa: u8 },
SetStyleBorderColor { obj: usize, color: lv_color_t },
SetStyleBorderWidth { obj: usize, value: i32 },
SetStyleBorderOpa { obj: usize, opa: u8 },
SetStylePadLeft  { obj: usize, value: i32 },
SetStylePadRight { obj: usize, value: i32 },
SetStylePadTop   { obj: usize, value: i32 },
SetStylePadBottom { obj: usize, value: i32 },
```

Update the mock setter functions to record them:

```rust
pub unsafe fn lv_obj_set_width(obj: *mut lv_obj_t, value: i32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetWidth { obj: obj as usize, value }));
}
pub unsafe fn lv_obj_set_height(obj: *mut lv_obj_t, value: i32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetHeight { obj: obj as usize, value }));
}
pub unsafe fn lv_obj_set_style_pad_top(obj: *mut lv_obj_t, value: i32, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStylePadTop { obj: obj as usize, value }));
}
pub unsafe fn lv_obj_set_style_pad_bottom(obj: *mut lv_obj_t, value: i32, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStylePadBottom { obj: obj as usize, value }));
}
pub unsafe fn lv_obj_set_style_pad_left(obj: *mut lv_obj_t, value: i32, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStylePadLeft { obj: obj as usize, value }));
}
pub unsafe fn lv_obj_set_style_pad_right(obj: *mut lv_obj_t, value: i32, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStylePadRight { obj: obj as usize, value }));
}
pub unsafe fn lv_obj_set_style_bg_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleBgColor { obj: obj as usize, color }));
}
pub unsafe fn lv_obj_set_style_bg_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleBgOpa { obj: obj as usize, opa }));
}
pub unsafe fn lv_obj_set_style_text_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleTextColor { obj: obj as usize, color }));
}
pub unsafe fn lv_obj_set_style_text_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleTextOpa { obj: obj as usize, opa }));
}
pub unsafe fn lv_obj_set_style_border_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleBorderColor { obj: obj as usize, color }));
}
pub unsafe fn lv_obj_set_style_border_width(obj: *mut lv_obj_t, value: i32, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleBorderWidth { obj: obj as usize, value }));
}
pub unsafe fn lv_obj_set_style_border_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleBorderOpa { obj: obj as usize, opa }));
}
```

- [ ] **Step 5: Run the focused tests to verify they pass**

Run:

```bash
cargo test color_is_copyable_for_style_structs mock_records_radio_list_style_primitives --quiet
```

Expected: both tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/lvgl/color.rs src/c_bindings.rs
git commit -m "test: add radio list mock style support" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

## Task 2: Construct the RadioButtonList Object Tree

**Files:**
- Create: `src/lvgl/radiobuttonlist/types.rs`
- Create: `src/lvgl/radiobuttonlist/tree.rs`
- Create: `src/lvgl/radiobuttonlist/style.rs`
- Create: `src/lvgl/radiobuttonlist/trampolines.rs`
- Create: `src/lvgl/radiobuttonlist/mod.rs`
- Modify: `src/lvgl/mod.rs`
- Modify: `src/lvgl/prelude.rs`

- [ ] **Step 1: Write failing construction and export tests**

Create `src/lvgl/radiobuttonlist/mod.rs` with public module declarations, an empty public type skeleton, and these tests:

```rust
mod types;
mod tree;
mod style;
mod trampolines;

pub use types::{RadioButtonEvent, RadioButtonListConfig, RadioButtonListStyle, RadioIndicatorStyle};

use crate::c_bindings;
use super::widget::{LvObj, Widget};

pub struct RadioButtonList {
    obj: LvObj,
}

impl Widget for RadioButtonList {
    fn lv_obj(&self) -> &LvObj { &self.obj }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::screen::Screen;

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_builds_root_row_indicator_and_label_for_each_option() {
        let p = parent();
        let list = RadioButtonList::new(&p, &["First", "Second"]);
        assert_eq!(list.len(), 2);

        let calls = spy_drain();
        let obj_creates = calls.iter().filter(|c| matches!(c, LvCall::ObjCreate { .. })).count();
        let label_creates = calls.iter().filter(|c| matches!(c, LvCall::LabelCreate { .. })).count();
        assert_eq!(obj_creates, 5, "root + 2 rows + 2 indicators, got {calls:?}");
        assert_eq!(label_creates, 2, "one label per option, got {calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"First\0")), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"Second\0")), "{calls:?}");
    }

    #[test]
    fn default_layout_sets_column_root_and_fixed_row_geometry() {
        let p = parent();
        let _list = RadioButtonList::new(&p, &["One"]);

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetFlexFlow { flow: 1, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 100, h: 44, .. })), "{calls:?}");
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 18, h: 18, .. })), "{calls:?}");
    }

    #[test]
    #[should_panic(expected = "RadioButtonList requires at least one option")]
    fn empty_options_panic() {
        let p = parent();
        let _ = RadioButtonList::new(&p, &[]);
    }
}
```

Modify `src/lvgl/mod.rs`:

```rust
mod radiobuttonlist;
pub use self::radiobuttonlist::{
    RadioButtonEvent, RadioButtonList, RadioButtonListConfig, RadioButtonListStyle,
    RadioIndicatorStyle,
};
```

Modify `src/lvgl/prelude.rs`:

```rust
pub use super::radiobuttonlist::{
    RadioButtonEvent, RadioButtonList, RadioButtonListConfig, RadioButtonListStyle,
    RadioIndicatorStyle,
};
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run:

```bash
cargo test radiobuttonlist::tests::new_builds_root_row_indicator_and_label_for_each_option radiobuttonlist::tests::default_layout_sets_column_root_and_fixed_row_geometry radiobuttonlist::tests::empty_options_panic --quiet
```

Expected: compile fails because `RadioButtonList::new`, `len`, and supporting modules/types are not implemented.

- [ ] **Step 3: Define public types and defaults**

Create `src/lvgl/radiobuttonlist/types.rs`:

```rust
use super::super::{Color, CornerRadius};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RadioButtonEvent<'a> {
    pub index: usize,
    pub label: &'a str,
}

#[derive(Copy, Clone)]
pub struct RadioButtonListStyle {
    pub bg_color: Option<Color>,
    pub bg_opa: Option<u8>,
    pub border_color: Option<Color>,
    pub border_width: Option<i32>,
    pub border_opa: Option<u8>,
    pub radius: Option<CornerRadius>,
    pub text_color: Option<Color>,
    pub text_opa: Option<u8>,
}

impl Default for RadioButtonListStyle {
    fn default() -> Self {
        Self {
            bg_color: None,
            bg_opa: None,
            border_color: None,
            border_width: None,
            border_opa: None,
            radius: None,
            text_color: None,
            text_opa: None,
        }
    }
}

#[derive(Copy, Clone)]
pub struct RadioIndicatorStyle {
    pub bg_color: Option<Color>,
    pub bg_opa: Option<u8>,
    pub border_color: Option<Color>,
    pub border_width: Option<i32>,
    pub border_opa: Option<u8>,
    pub radius: Option<CornerRadius>,
}

impl Default for RadioIndicatorStyle {
    fn default() -> Self {
        Self {
            bg_color: None,
            bg_opa: Some(0),
            border_color: None,
            border_width: Some(1),
            border_opa: None,
            radius: Some(CornerRadius::Full),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RadioButtonListConfig {
    pub row_height: i32,
    pub gap: i32,
    pub pad_h: i32,
    pub pad_v: i32,
    pub indicator_size: i32,
    pub indicator_label_gap: i32,
}

impl Default for RadioButtonListConfig {
    fn default() -> Self {
        Self {
            row_height: 44,
            gap: 8,
            pad_h: 12,
            pad_v: 10,
            indicator_size: 18,
            indicator_label_gap: 12,
        }
    }
}

pub(crate) fn assert_valid_options(labels: &[&str]) {
    assert!(!labels.is_empty(), "RadioButtonList requires at least one option");
}

pub(crate) fn assert_valid_config(cfg: RadioButtonListConfig) {
    assert!(cfg.row_height > 0, "RadioButtonList row height must be positive, got {}", cfg.row_height);
    assert!(cfg.indicator_size > 0, "RadioButtonList indicator size must be positive, got {}", cfg.indicator_size);
    assert!(cfg.gap >= 0, "RadioButtonList gap must be non-negative, got {}", cfg.gap);
    assert!(cfg.indicator_label_gap >= 0, "RadioButtonList indicator-label gap must be non-negative, got {}", cfg.indicator_label_gap);
}
```

- [ ] **Step 4: Build the LVGL object tree**

Create `src/lvgl/radiobuttonlist/tree.rs`:

```rust
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_char;

use crate::c_bindings;
use super::types::RadioButtonListConfig;

pub(crate) struct RowWidgets {
    pub row: *mut c_bindings::lv_obj_t,
    pub indicator: *mut c_bindings::lv_obj_t,
    pub label: *mut c_bindings::lv_obj_t,
}

pub(crate) struct Tree {
    pub root: *mut c_bindings::lv_obj_t,
    pub rows: Vec<RowWidgets>,
}

pub(crate) unsafe fn build(
    parent: *mut c_bindings::lv_obj_t,
    labels: &[String],
    cfg: RadioButtonListConfig,
) -> Tree {
    let root = unsafe { c_bindings::lv_obj_create(parent) };
    if root.is_null() {
        panic!("lv_obj_create returned null for RadioButtonList root");
    }

    unsafe {
        c_bindings::lv_obj_set_flex_flow(root, super::super::FlexFlow::Column as u32);
        c_bindings::lv_obj_set_style_pad_row(root, cfg.gap, 0);
        c_bindings::lv_obj_set_style_pad_column(root, 0, 0);
    }

    let mut rows = Vec::with_capacity(labels.len());
    for label_text in labels {
        let row = unsafe { c_bindings::lv_obj_create(root) };
        if row.is_null() {
            panic!("lv_obj_create returned null for RadioButtonList row");
        }
        let indicator = unsafe { c_bindings::lv_obj_create(row) };
        if indicator.is_null() {
            panic!("lv_obj_create returned null for RadioButtonList indicator");
        }
        let label = unsafe { c_bindings::lv_label_create(row) };
        if label.is_null() {
            panic!("lv_label_create returned null for RadioButtonList label");
        }

        let label_buf = super::super::util::to_null_terminated(label_text);
        unsafe {
            c_bindings::lv_obj_add_flag(row, super::super::LvObjFlag::CLICKABLE.0);
            c_bindings::lv_obj_set_flex_flow(row, super::super::FlexFlow::Row as u32);
            c_bindings::lv_obj_set_size(row, c_bindings::lv_pct(100), cfg.row_height);
            c_bindings::lv_obj_set_style_pad_left(row, cfg.pad_h, 0);
            c_bindings::lv_obj_set_style_pad_right(row, cfg.pad_h, 0);
            c_bindings::lv_obj_set_style_pad_top(row, cfg.pad_v, 0);
            c_bindings::lv_obj_set_style_pad_bottom(row, cfg.pad_v, 0);
            c_bindings::lv_obj_set_style_pad_column(row, cfg.indicator_label_gap, 0);
            c_bindings::lv_obj_set_size(indicator, cfg.indicator_size, cfg.indicator_size);
            c_bindings::lv_label_set_text(label, label_buf.as_ptr() as *const c_char);
        }

        rows.push(RowWidgets { row, indicator, label });
    }

    Tree { root, rows }
}
```

Create `src/lvgl/radiobuttonlist/style.rs` with empty safe functions used by later tasks:

```rust
use crate::c_bindings;
use super::tree::RowWidgets;
use super::types::{RadioButtonListStyle, RadioIndicatorStyle};

pub(crate) fn apply_row_style(_row: *mut c_bindings::lv_obj_t, _style: RadioButtonListStyle) {}
pub(crate) fn apply_label_style(_label: *mut c_bindings::lv_obj_t, _style: RadioButtonListStyle) {}
pub(crate) fn apply_indicator_style(_indicator: *mut c_bindings::lv_obj_t, _style: RadioIndicatorStyle) {}
pub(crate) fn apply_row_visuals(_widgets: &RowWidgets, _selected: bool, _enabled: bool) {}
```

Create `src/lvgl/radiobuttonlist/trampolines.rs` with inert registration used by later tasks:

```rust
use crate::c_bindings;

pub(crate) struct RowCtx {
    pub list: *mut super::RadioButtonList,
    pub index: usize,
}

pub(crate) unsafe fn register_row(
    _row: *mut c_bindings::lv_obj_t,
    _ctx: *mut RowCtx,
) {
}

pub(crate) unsafe fn unregister_row(
    _row: *mut c_bindings::lv_obj_t,
    _ctx: *mut RowCtx,
) {
}
```

- [ ] **Step 5: Implement `RadioButtonList::new`, `len`, and object ownership**

Replace the skeleton in `src/lvgl/radiobuttonlist/mod.rs` with:

```rust
mod types;
mod tree;
mod style;
mod trampolines;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use crate::c_bindings;
use super::widget::{LvObj, Widget};

pub use types::{RadioButtonEvent, RadioButtonListConfig, RadioButtonListStyle, RadioIndicatorStyle};

type ChangeCallback = Box<dyn for<'a> FnMut(RadioButtonEvent<'a>)>;

pub struct RadioButtonList {
    obj: LvObj,
    labels: Vec<String>,
    tree: tree::Tree,
    row_ctxs: Vec<Box<trampolines::RowCtx>>,
    enabled: Vec<bool>,
    selected: Option<usize>,
    cfg: RadioButtonListConfig,
    row_style: RadioButtonListStyle,
    selected_row_style: RadioButtonListStyle,
    label_style: RadioButtonListStyle,
    indicator_style: RadioIndicatorStyle,
    selected_indicator_style: RadioIndicatorStyle,
    callback: RefCell<Option<ChangeCallback>>,
}

impl Widget for RadioButtonList {
    fn lv_obj(&self) -> &LvObj { &self.obj }
}

impl RadioButtonList {
    pub fn new(parent: &impl Widget, labels: &[&str]) -> Box<Self> {
        Self::with_config(parent, labels, RadioButtonListConfig::default())
    }

    pub fn with_config(
        parent: &impl Widget,
        labels: &[&str],
        cfg: RadioButtonListConfig,
    ) -> Box<Self> {
        types::assert_valid_options(labels);
        types::assert_valid_config(cfg);

        let owned_labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        let tree = unsafe { tree::build(parent.lv_obj().raw(), &owned_labels, cfg) };
        let root = tree.root;
        let enabled = alloc::vec![true; owned_labels.len()];

        let mut list = Box::new(Self {
            obj: LvObj::from_raw(root),
            labels: owned_labels,
            tree,
            row_ctxs: Vec::new(),
            enabled,
            selected: None,
            cfg,
            row_style: RadioButtonListStyle::default(),
            selected_row_style: RadioButtonListStyle::default(),
            label_style: RadioButtonListStyle::default(),
            indicator_style: RadioIndicatorStyle::default(),
            selected_indicator_style: RadioIndicatorStyle {
                bg_opa: Some(255),
                ..RadioIndicatorStyle::default()
            },
            callback: RefCell::new(None),
        });

        let list_ptr: *mut RadioButtonList = list.as_mut();
        for (index, widgets) in list.tree.rows.iter().enumerate() {
            let mut ctx = Box::new(trampolines::RowCtx { list: list_ptr, index });
            unsafe { trampolines::register_row(widgets.row, ctx.as_mut() as *mut _) };
            list.row_ctxs.push(ctx);
        }

        list
    }

    #[must_use]
    pub fn len(&self) -> usize { self.labels.len() }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.labels.is_empty() }
}

impl Drop for RadioButtonList {
    fn drop(&mut self) {
        for (widgets, ctx) in self.tree.rows.iter().zip(self.row_ctxs.iter_mut()) {
            unsafe { trampolines::unregister_row(widgets.row, ctx.as_mut() as *mut _) };
        }
    }
}
```

- [ ] **Step 6: Run the focused construction tests**

Run:

```bash
cargo test radiobuttonlist::tests::new_builds_root_row_indicator_and_label_for_each_option radiobuttonlist::tests::default_layout_sets_column_root_and_fixed_row_geometry radiobuttonlist::tests::empty_options_panic --quiet
```

Expected: all three tests pass.

- [ ] **Step 7: Run module compilation through exports**

Run:

```bash
cargo test --lib --quiet
```

Expected: all existing library tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/lvgl/radiobuttonlist src/lvgl/mod.rs src/lvgl/prelude.rs
git commit -m "feat: add radio button list tree" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

## Task 3: Styling and Programmatic Selection

**Files:**
- Modify: `src/lvgl/radiobuttonlist/style.rs`
- Modify: `src/lvgl/radiobuttonlist/mod.rs`
- Modify: `src/lvgl/radiobuttonlist/types.rs`

- [ ] **Step 1: Write failing tests for style setters and selection**

Add these tests to the `radiobuttonlist::tests` module:

```rust
#[test]
fn set_selected_updates_state_and_checked_indicator_visuals() {
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One", "Two"]);
    spy_drain();

    list.selected_indicator_style(RadioIndicatorStyle {
        bg_color: Some(Color::hex(0xFF6600)),
        bg_opa: Some(255),
        border_color: Some(Color::hex(0xFF6600)),
        border_width: Some(2),
        border_opa: Some(255),
        radius: Some(CornerRadius::Full),
    });
    list.set_selected(Some(1));

    assert_eq!(list.selected(), Some(1));
    let calls = spy_drain();
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBgOpa { opa: 255, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBorderWidth { value: 2, .. })), "{calls:?}");
}

#[test]
fn set_selected_none_clears_selection() {
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One", "Two"]);

    list.set_selected(Some(0));
    list.set_selected(None);

    assert_eq!(list.selected(), None);
}

#[test]
#[should_panic(expected = "RadioButtonList selection index out of range: 2 >= 2")]
fn set_selected_out_of_range_panics() {
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One", "Two"]);
    list.set_selected(Some(2));
}

#[test]
fn row_style_setter_applies_to_rows() {
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One"]);
    spy_drain();

    list.row_style(RadioButtonListStyle {
        bg_color: Some(Color::hex(0xFFFFFF)),
        bg_opa: Some(255),
        border_color: Some(Color::hex(0x203844)),
        border_width: Some(1),
        border_opa: Some(255),
        radius: Some(CornerRadius::Px(8)),
        text_color: None,
        text_opa: None,
    });

    let calls = spy_drain();
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBgOpa { opa: 255, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleRadius { value: 8, .. })), "{calls:?}");
}
```

Add the imports at the top of the tests module:

```rust
use crate::lvgl::{Color, CornerRadius};
```

- [ ] **Step 2: Run focused tests to verify they fail**

Run:

```bash
cargo test radiobuttonlist::tests::set_selected_updates_state_and_checked_indicator_visuals radiobuttonlist::tests::set_selected_none_clears_selection radiobuttonlist::tests::set_selected_out_of_range_panics radiobuttonlist::tests::row_style_setter_applies_to_rows --quiet
```

Expected: compile fails because style setter and selection APIs are not implemented.

- [ ] **Step 3: Implement style application**

Replace `src/lvgl/radiobuttonlist/style.rs` with:

```rust
use crate::c_bindings;
use super::tree::RowWidgets;
use super::types::{RadioButtonListStyle, RadioIndicatorStyle};

pub(crate) fn apply_row_style(row: *mut c_bindings::lv_obj_t, style: RadioButtonListStyle) {
    unsafe {
        if let Some(c) = style.bg_color { c_bindings::lv_obj_set_style_bg_color(row, c.to_lv(), 0); }
        if let Some(opa) = style.bg_opa { c_bindings::lv_obj_set_style_bg_opa(row, opa, 0); }
        if let Some(c) = style.border_color { c_bindings::lv_obj_set_style_border_color(row, c.to_lv(), 0); }
        if let Some(w) = style.border_width { c_bindings::lv_obj_set_style_border_width(row, w, 0); }
        if let Some(opa) = style.border_opa { c_bindings::lv_obj_set_style_border_opa(row, opa, 0); }
        if let Some(r) = style.radius { c_bindings::lv_obj_set_style_radius(row, r.into_lv_value(), 0); }
    }
}

pub(crate) fn apply_label_style(label: *mut c_bindings::lv_obj_t, style: RadioButtonListStyle) {
    unsafe {
        if let Some(c) = style.text_color { c_bindings::lv_obj_set_style_text_color(label, c.to_lv(), 0); }
        if let Some(opa) = style.text_opa { c_bindings::lv_obj_set_style_text_opa(label, opa, 0); }
    }
}

pub(crate) fn apply_indicator_style(indicator: *mut c_bindings::lv_obj_t, style: RadioIndicatorStyle) {
    unsafe {
        if let Some(c) = style.bg_color { c_bindings::lv_obj_set_style_bg_color(indicator, c.to_lv(), 0); }
        if let Some(opa) = style.bg_opa { c_bindings::lv_obj_set_style_bg_opa(indicator, opa, 0); }
        if let Some(c) = style.border_color { c_bindings::lv_obj_set_style_border_color(indicator, c.to_lv(), 0); }
        if let Some(w) = style.border_width { c_bindings::lv_obj_set_style_border_width(indicator, w, 0); }
        if let Some(opa) = style.border_opa { c_bindings::lv_obj_set_style_border_opa(indicator, opa, 0); }
        if let Some(r) = style.radius { c_bindings::lv_obj_set_style_radius(indicator, r.into_lv_value(), 0); }
    }
}

pub(crate) fn apply_visuals(
    widgets: &RowWidgets,
    selected: bool,
    enabled: bool,
    row_style: RadioButtonListStyle,
    selected_row_style: RadioButtonListStyle,
    label_style: RadioButtonListStyle,
    indicator_style: RadioIndicatorStyle,
    selected_indicator_style: RadioIndicatorStyle,
) {
    apply_row_style(widgets.row, row_style);
    apply_label_style(widgets.label, label_style);
    apply_indicator_style(widgets.indicator, indicator_style);
    if selected {
        apply_row_style(widgets.row, selected_row_style);
        apply_indicator_style(widgets.indicator, selected_indicator_style);
    }
    if enabled {
        unsafe { c_bindings::lv_obj_remove_state(widgets.row, super::super::LvState::DISABLED.0); }
    } else {
        unsafe { c_bindings::lv_obj_add_state(widgets.row, super::super::LvState::DISABLED.0); }
    }
}
```

- [ ] **Step 4: Add selection and style APIs**

Add these methods to `impl RadioButtonList` in `src/lvgl/radiobuttonlist/mod.rs`:

```rust
#[must_use]
pub fn selected(&self) -> Option<usize> { self.selected }

pub fn set_selected(&mut self, selected: Option<usize>) -> &mut Self {
    if let Some(index) = selected {
        self.assert_index(index, "selection");
    }
    let old = self.selected;
    self.selected = selected;
    if let Some(index) = old { self.refresh_row(index); }
    if let Some(index) = selected {
        if Some(index) != old { self.refresh_row(index); }
        if Some(index) == old { self.refresh_row(index); }
    }
    self
}

pub fn row_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
    self.row_style = style;
    self.refresh_all_rows();
    self
}

pub fn selected_row_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
    self.selected_row_style = style;
    self.refresh_all_rows();
    self
}

pub fn indicator_style(&mut self, style: RadioIndicatorStyle) -> &mut Self {
    self.indicator_style = style;
    self.refresh_all_rows();
    self
}

pub fn selected_indicator_style(&mut self, style: RadioIndicatorStyle) -> &mut Self {
    self.selected_indicator_style = style;
    self.refresh_all_rows();
    self
}

pub fn label_style(&mut self, style: RadioButtonListStyle) -> &mut Self {
    self.label_style = style;
    self.refresh_all_rows();
    self
}

fn assert_index(&self, index: usize, purpose: &str) {
    assert!(
        index < self.labels.len(),
        "RadioButtonList {} index out of range: {} >= {}",
        purpose,
        index,
        self.labels.len()
    );
}

fn refresh_row(&self, index: usize) {
    let widgets = &self.tree.rows[index];
    style::apply_visuals(
        widgets,
        self.selected == Some(index),
        self.enabled[index],
        self.row_style,
        self.selected_row_style,
        self.label_style,
        self.indicator_style,
        self.selected_indicator_style,
    );
}

fn refresh_all_rows(&self) {
    for index in 0..self.labels.len() {
        self.refresh_row(index);
    }
}
```

- [ ] **Step 5: Run focused selection/style tests**

Run:

```bash
cargo test radiobuttonlist::tests::set_selected_updates_state_and_checked_indicator_visuals radiobuttonlist::tests::set_selected_none_clears_selection radiobuttonlist::tests::set_selected_out_of_range_panics radiobuttonlist::tests::row_style_setter_applies_to_rows --quiet
```

Expected: all four tests pass.

- [ ] **Step 6: Run full library tests**

Run:

```bash
cargo test --lib --quiet
```

Expected: all library tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/lvgl/radiobuttonlist
git commit -m "feat: add radio button list selection styling" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

## Task 4: Enabled State and Click Callback Dispatch

**Files:**
- Modify: `src/lvgl/radiobuttonlist/trampolines.rs`
- Modify: `src/lvgl/radiobuttonlist/mod.rs`

- [ ] **Step 1: Write failing tests for enabled state and callbacks**

Add these tests to `radiobuttonlist::tests`:

```rust
#[test]
fn set_enabled_false_marks_row_disabled_and_preserves_selection() {
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One", "Two"]);
    list.set_selected(Some(1));
    spy_drain();

    list.set_enabled(1, false);

    assert_eq!(list.selected(), Some(1));
    assert!(!list.is_enabled(1));
    let calls = spy_drain();
    assert!(calls.iter().any(|c| matches!(c, LvCall::AddState { state, .. } if *state == crate::lvgl::LvState::DISABLED.0)), "{calls:?}");
}

#[test]
#[should_panic(expected = "RadioButtonList enabled index out of range: 2 >= 2")]
fn set_enabled_out_of_range_panics() {
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One", "Two"]);
    list.set_enabled(2, false);
}

#[test]
fn clicking_enabled_row_selects_then_calls_callback() {
    use crate::c_bindings::{LV_EVENT_CLICKED, spy_emit_event};
    use core::sync::atomic::{AtomicUsize, Ordering};

    static INDEX: AtomicUsize = AtomicUsize::new(usize::MAX);
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One", "Two"]);
    list.on_changed(|event| {
        assert_eq!(event.label, "Two");
        INDEX.store(event.index, Ordering::SeqCst);
    });

    let row = list.debug_row_raw_for_test(1);
    spy_emit_event(row as *mut _, LV_EVENT_CLICKED);

    assert_eq!(list.selected(), Some(1));
    assert_eq!(INDEX.load(Ordering::SeqCst), 1);
}

#[test]
fn clicking_disabled_row_does_not_select_or_call_callback() {
    use crate::c_bindings::{LV_EVENT_CLICKED, spy_emit_event};
    use core::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One", "Two"]);
    list.set_enabled(1, false);
    list.on_changed(|_event| {
        CALLS.fetch_add(1, Ordering::SeqCst);
    });

    let row = list.debug_row_raw_for_test(1);
    spy_emit_event(row as *mut _, LV_EVENT_CLICKED);

    assert_eq!(list.selected(), None);
    assert_eq!(CALLS.load(Ordering::SeqCst), 0);
}
```

- [ ] **Step 2: Run focused tests to verify they fail**

Run:

```bash
cargo test radiobuttonlist::tests::set_enabled_false_marks_row_disabled_and_preserves_selection radiobuttonlist::tests::set_enabled_out_of_range_panics radiobuttonlist::tests::clicking_enabled_row_selects_then_calls_callback radiobuttonlist::tests::clicking_disabled_row_does_not_select_or_call_callback --quiet
```

Expected: compile fails because enabled APIs, callback API, real trampoline registration, and test row access are not implemented.

- [ ] **Step 3: Implement real row trampolines**

Replace `src/lvgl/radiobuttonlist/trampolines.rs` with:

```rust
use crate::c_bindings;

pub(crate) struct RowCtx {
    pub list: *mut super::RadioButtonList,
    pub index: usize,
}

unsafe extern "C" fn on_row_clicked(e: *mut c_bindings::lv_event_t) {
    let user_data = unsafe { c_bindings::lv_event_get_user_data(e) } as *mut RowCtx;
    if user_data.is_null() { return; }
    let ctx = unsafe { &mut *user_data };
    if ctx.list.is_null() { return; }
    let list = unsafe { &mut *ctx.list };
    list.handle_row_clicked(ctx.index);
}

pub(crate) unsafe fn register_row(
    row: *mut c_bindings::lv_obj_t,
    ctx: *mut RowCtx,
) {
    unsafe {
        c_bindings::lv_obj_add_event_cb(
            row,
            Some(on_row_clicked),
            c_bindings::LV_EVENT_CLICKED,
            ctx as *mut core::ffi::c_void,
        );
    }
}

pub(crate) unsafe fn unregister_row(
    row: *mut c_bindings::lv_obj_t,
    ctx: *mut RowCtx,
) {
    unsafe {
        c_bindings::lv_obj_remove_event_cb_with_user_data(
            row,
            Some(on_row_clicked),
            ctx as *mut core::ffi::c_void,
        );
    }
}
```

- [ ] **Step 4: Add enabled and callback methods**

Add these methods to `impl RadioButtonList`:

```rust
pub fn set_enabled(&mut self, index: usize, enabled: bool) -> &mut Self {
    self.assert_index(index, "enabled");
    self.enabled[index] = enabled;
    self.refresh_row(index);
    self
}

#[must_use]
pub fn is_enabled(&self, index: usize) -> bool {
    self.assert_index(index, "enabled");
    self.enabled[index]
}

pub fn on_changed<F>(&mut self, f: F) -> &mut Self
where
    F: for<'a> FnMut(RadioButtonEvent<'a>) + 'static,
{
    *self.callback.borrow_mut() = Some(Box::new(f));
    self
}

pub(crate) fn handle_row_clicked(&mut self, index: usize) {
    self.assert_index(index, "selection");
    if !self.enabled[index] {
        return;
    }
    self.set_selected(Some(index));
    let label = self.labels[index].as_str();
    let event = RadioButtonEvent { index, label };
    let cb = self.callback.get_mut();
    if let Some(f) = cb.as_mut() {
        f(event);
    }
}

#[cfg(test)]
pub fn debug_row_raw_for_test(&self, index: usize) -> usize {
    self.assert_index(index, "debug row");
    self.tree.rows[index].row as usize
}
```

- [ ] **Step 5: Run focused enabled/callback tests**

Run:

```bash
cargo test radiobuttonlist::tests::set_enabled_false_marks_row_disabled_and_preserves_selection radiobuttonlist::tests::set_enabled_out_of_range_panics radiobuttonlist::tests::clicking_enabled_row_selects_then_calls_callback radiobuttonlist::tests::clicking_disabled_row_does_not_select_or_call_callback --quiet
```

Expected: all four tests pass.

- [ ] **Step 6: Run full library tests**

Run:

```bash
cargo test --lib --quiet
```

Expected: all library tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/lvgl/radiobuttonlist
git commit -m "feat: add radio button list callbacks" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

## Task 5: Configuration Setters and Validation Coverage

**Files:**
- Modify: `src/lvgl/radiobuttonlist/mod.rs`
- Modify: `src/lvgl/radiobuttonlist/types.rs`

- [ ] **Step 1: Write failing tests for configuration methods**

Add these tests to `radiobuttonlist::tests`:

```rust
#[test]
fn with_config_applies_custom_row_height_gap_padding_and_indicator_size() {
    let p = parent();
    let _list = RadioButtonList::with_config(&p, &["One"], RadioButtonListConfig {
        row_height: 72,
        gap: 9,
        pad_h: 21,
        pad_v: 22,
        indicator_size: 24,
        indicator_label_gap: 15,
    });

    let calls = spy_drain();
    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 100, h: 72, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 24, h: 24, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadRow { value: 9, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadLeft { value: 21, .. })), "{calls:?}");
    assert!(calls.iter().any(|c| matches!(c, LvCall::SetStylePadTop { value: 22, .. })), "{calls:?}");
}

#[test]
#[should_panic(expected = "RadioButtonList row height must be positive, got 0")]
fn zero_row_height_panics() {
    let p = parent();
    let _ = RadioButtonList::with_config(&p, &["One"], RadioButtonListConfig {
        row_height: 0,
        ..RadioButtonListConfig::default()
    });
}

#[test]
#[should_panic(expected = "RadioButtonList indicator size must be positive, got 0")]
fn zero_indicator_size_panics() {
    let p = parent();
    let _ = RadioButtonList::with_config(&p, &["One"], RadioButtonListConfig {
        indicator_size: 0,
        ..RadioButtonListConfig::default()
    });
}

#[test]
fn chaining_core_setters_returns_mut_self() {
    let p = parent();
    let mut list = RadioButtonList::new(&p, &["One", "Two"]);
    let ptr: *const RadioButtonList = &*list;
    let ret = list
        .row_height(60)
        .gap(4)
        .row_padding(14, 15)
        .indicator_size(20)
        .indicator_label_gap(10)
        .set_selected(Some(0));
    assert!(core::ptr::eq(ret as *const RadioButtonList, ptr));
}
```

- [ ] **Step 2: Add `SetStylePadRow` and `SetStylePadColumn` spy variants**

Extend `LvCall` in `src/c_bindings.rs`:

```rust
SetStylePadRow { obj: usize, value: i32 },
SetStylePadColumn { obj: usize, value: i32 },
```

Update mock functions:

```rust
pub unsafe fn lv_obj_set_style_pad_row(obj: *mut lv_obj_t, value: i32, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStylePadRow { obj: obj as usize, value }));
}
pub unsafe fn lv_obj_set_style_pad_column(obj: *mut lv_obj_t, value: i32, _: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStylePadColumn { obj: obj as usize, value }));
}
```

- [ ] **Step 3: Run focused tests to verify they fail**

Run:

```bash
cargo test radiobuttonlist::tests::with_config_applies_custom_row_height_gap_padding_and_indicator_size radiobuttonlist::tests::zero_row_height_panics radiobuttonlist::tests::zero_indicator_size_panics radiobuttonlist::tests::chaining_core_setters_returns_mut_self --quiet
```

Expected: compile fails because configuration mutation setters are not implemented.

- [ ] **Step 4: Add configuration mutation setters**

Add these methods to `impl RadioButtonList`:

```rust
pub fn row_height(&mut self, row_height: i32) -> &mut Self {
    let mut cfg = self.cfg;
    cfg.row_height = row_height;
    types::assert_valid_config(cfg);
    self.cfg = cfg;
    for widgets in &self.tree.rows {
        unsafe { c_bindings::lv_obj_set_size(widgets.row, c_bindings::lv_pct(100), row_height); }
    }
    self
}

pub fn gap(&mut self, gap: i32) -> &mut Self {
    let mut cfg = self.cfg;
    cfg.gap = gap;
    types::assert_valid_config(cfg);
    self.cfg = cfg;
    unsafe { c_bindings::lv_obj_set_style_pad_row(self.tree.root, gap, 0); }
    self
}

pub fn row_padding(&mut self, horizontal: i32, vertical: i32) -> &mut Self {
    let mut cfg = self.cfg;
    cfg.pad_h = horizontal;
    cfg.pad_v = vertical;
    types::assert_valid_config(cfg);
    self.cfg = cfg;
    for widgets in &self.tree.rows {
        unsafe {
            c_bindings::lv_obj_set_style_pad_left(widgets.row, horizontal, 0);
            c_bindings::lv_obj_set_style_pad_right(widgets.row, horizontal, 0);
            c_bindings::lv_obj_set_style_pad_top(widgets.row, vertical, 0);
            c_bindings::lv_obj_set_style_pad_bottom(widgets.row, vertical, 0);
        }
    }
    self
}

pub fn indicator_size(&mut self, indicator_size: i32) -> &mut Self {
    let mut cfg = self.cfg;
    cfg.indicator_size = indicator_size;
    types::assert_valid_config(cfg);
    self.cfg = cfg;
    for widgets in &self.tree.rows {
        unsafe { c_bindings::lv_obj_set_size(widgets.indicator, indicator_size, indicator_size); }
    }
    self
}

pub fn indicator_label_gap(&mut self, gap: i32) -> &mut Self {
    let mut cfg = self.cfg;
    cfg.indicator_label_gap = gap;
    types::assert_valid_config(cfg);
    self.cfg = cfg;
    for widgets in &self.tree.rows {
        unsafe { c_bindings::lv_obj_set_style_pad_column(widgets.row, gap, 0); }
    }
    self
}
```

- [ ] **Step 5: Run focused configuration tests**

Run:

```bash
cargo test radiobuttonlist::tests::with_config_applies_custom_row_height_gap_padding_and_indicator_size radiobuttonlist::tests::zero_row_height_panics radiobuttonlist::tests::zero_indicator_size_panics radiobuttonlist::tests::chaining_core_setters_returns_mut_self --quiet
```

Expected: all four tests pass.

- [ ] **Step 6: Run full library tests**

Run:

```bash
cargo test --lib --quiet
```

Expected: all library tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/lvgl/radiobuttonlist src/c_bindings.rs
git commit -m "feat: add radio button list configuration" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

## Task 6: Reference Documentation and Playground

**Files:**
- Modify: `DSL_REFERENCE.md`
- Modify: `DSL_PLAYGROUND.html`

- [ ] **Step 1: Add reference documentation**

Add `RadioButtonList` to the widget list/table of contents in `DSL_REFERENCE.md`, then add this section near other widget-specific sections:

```markdown
## RadioButtonList

`RadioButtonList` is a composite widget for choosing one option from a vertical
list. It creates a root container, one clickable row per option, a circular
indicator for each row, and a label for each runtime `&str` option. LVGL copies
label text when rows are created, so option labels do not need to be static C
string maps.

```rust
use lvgl_dsl::lvgl::prelude::*;

let mut reasons = RadioButtonList::new(&screen, &[
    "Choose another locker",
    "Cancel placement",
    "Locker didn't open",
    "Locker is occupied",
]);

reasons
    .row_height(44)
    .gap(8)
    .row_padding(12, 10)
    .indicator_size(18)
    .indicator_label_gap(12)
    .set_selected(Some(0))
    .on_changed(|event| {
        let _index = event.index;
        let _label = event.label;
    });
```

Selection is single-choice. Clicking an enabled row selects it before
`on_changed` runs. Disabled rows remain visible but do not select and do not
call the callback.

| Method | Purpose |
| --- | --- |
| `new(parent, labels)` | Build a list from runtime `&str` labels. |
| `with_config(parent, labels, config)` | Build with explicit row height, gaps, padding, and indicator size. |
| `row_height(px)` | Set every row to a fixed pixel height. |
| `gap(px)` | Set vertical space between rows. |
| `row_padding(horizontal, vertical)` | Set row internal padding. |
| `indicator_size(px)` | Set circular indicator width and height. |
| `indicator_label_gap(px)` | Set horizontal gap between indicator and label. |
| `set_selected(option)` | Select an index or clear selection with `None`. |
| `selected()` | Return the current selected index. |
| `set_enabled(index, enabled)` | Enable or disable one row. |
| `is_enabled(index)` | Return whether a row is enabled. |
| `on_changed(callback)` | Install a typed callback receiving `RadioButtonEvent`. |

Use `RadioButtonListStyle` and `RadioIndicatorStyle` to apply product-specific
colors, borders, opacity, radius, and text colors. The widget does not hardcode a
Jetbeep visual theme.
```

- [ ] **Step 2: Add a playground section**

In `DSL_PLAYGROUND.html`, add a quick link named `RadioButtonList`. Add a section with controls for option labels, selected index, disabled indexes, row height, gap, indicator size, and generated Rust code. Use this generated-code shape:

```rust
let mut radio = RadioButtonList::new(&screen, &[
    "Items don't fit",
    "I don't need it anymore",
    "Other",
]);
radio
    .row_height(72)
    .gap(8)
    .row_padding(20, 12)
    .indicator_size(24)
    .indicator_label_gap(16)
    .set_selected(Some(0));
radio.set_enabled(2, false);
```

The preview should render ordinary HTML cards with a circular indicator so users
can understand the layout. Keep the section self-contained like existing
playground sections.

- [ ] **Step 3: Run documentation sanity checks**

Run:

```bash
grep -n "RadioButtonList" DSL_REFERENCE.md DSL_PLAYGROUND.html
cargo test --lib --quiet
```

Expected: `grep` prints matches from both files, and all library tests pass.

- [ ] **Step 4: Commit**

```bash
git add DSL_REFERENCE.md DSL_PLAYGROUND.html
git commit -m "docs: document radio button list" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

## Task 7: Final Validation

**Files:**
- Inspect all modified files from Tasks 1-6.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --check
```

Expected: success. If it fails, run `cargo fmt`, inspect the diff, and commit formatting together with the task that introduced the formatting changes.

- [ ] **Step 2: Run full test suite**

Run:

```bash
cargo test --quiet
```

Expected: all tests pass. Existing warnings from the baseline keyboard/accent code may remain; do not modify unrelated files to silence them.

- [ ] **Step 3: Inspect final diff**

Run:

```bash
git --no-pager status --short
git --no-pager log --oneline -8
git --no-pager diff --stat main...HEAD || git --no-pager diff --stat
```

Expected: only RadioButtonList-related source, docs, and planned mock/color support changes are present.

- [ ] **Step 4: Commit remaining validation changes**

If `cargo fmt` modified files or documentation edits were made after the previous commit, commit them:

```bash
git add src/lvgl/radiobuttonlist src/lvgl/color.rs src/lvgl/mod.rs src/lvgl/prelude.rs src/c_bindings.rs DSL_REFERENCE.md DSL_PLAYGROUND.html
git commit -m "chore: finalize radio button list widget" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

If `git status --short` is empty, do not create an empty commit.

## Self-Review

Spec coverage:

- Reusable `RadioButtonList` widget: Tasks 2-4.
- Runtime `&str` labels copied into LVGL labels: Task 2.
- Fixed row height, row gap, row padding, indicator size, indicator-label gap: Tasks 2 and 5.
- Low-level style structs/setters: Task 3.
- Automatic single selection and `selected`/`set_selected`: Task 3.
- Typed callback after auto-selection: Task 4.
- Per-option enabled state with disabled rows suppressing selection/callback: Task 4.
- Clear panics for invalid construction and indexes: Tasks 2, 3, and 5.
- Mock bindings, spy calls, unit tests: Tasks 1-5.
- Reference documentation and playground: Task 6.

Placeholder scan: no deferred sections, vague edge-case steps, or undefined method names remain.

Type consistency: public names are consistent across tasks: `RadioButtonList`, `RadioButtonEvent`, `RadioButtonListConfig`, `RadioButtonListStyle`, `RadioIndicatorStyle`, `set_selected`, `selected`, `set_enabled`, `is_enabled`, and `on_changed`.
