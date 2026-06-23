# PhoneFormatterField Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reusable `PhoneFormatterField` composite widget that formats digit-only input into phone/code display text and optionally renders a customizable left action slot.

**Architecture:** Implement the field as a composite root `Obj` containing a hidden-by-default left slot and a one-line `TextArea`. Formatting lives in a focused `FormatPreset` type, while live input normalization is handled by a value-changed LVGL trampoline using a boxed context and unregistering on drop.

**Tech Stack:** Rust 2024, `no_std` with `alloc`, LVGL v9.2 object/text-area/event/style APIs, existing mock `c_bindings` spy layer, Markdown reference docs, standalone HTML/CSS/JavaScript playground.

---

## File Structure

- Create `src/lvgl/phone_formatter_field.rs`: public `PhoneFormatterField`, `FormatPreset`, `LeftSlot`, `LeftSlotHandle`, formatting logic, composite construction, value-changed trampoline, left-slot APIs, and unit tests.
- Modify `src/lvgl/mod.rs`: add the `phone_formatter_field` module and public re-exports.
- Modify `src/lvgl/prelude.rs`: re-export the public phone-formatter API for `use lvgl_dsl::lvgl::prelude::*`.
- Modify `src/c_bindings.rs`: add mock spy variants/functions for width, height, padding, and style setters used by the composite widget tests.
- Modify `DSL_REFERENCE.md`: document `PhoneFormatterField`, `FormatPreset`, and `LeftSlot`.
- Modify `DSL_PLAYGROUND.html`: add a visual playground section for group and mask presets, left-slot controls, raw/formatted preview, and generated Rust DSL.

## Implementation Tasks

### Task 1: Add mock spy support for field layout and styles

**Files:**
- Modify: `src/c_bindings.rs`

- [ ] **Step 1: Write failing c_bindings spy tests**

Add these tests inside `src/c_bindings.rs` in `#[cfg(test)] mod tests`, after the existing event registry tests:

```rust
    #[test]
    fn phone_formatter_geometry_spy_records_width_and_height() {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };

        unsafe {
            lv_obj_set_width(obj, 72);
            lv_obj_set_height(obj, 56);
        }

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::ObjSetWidth { obj: recorded, w: 72 } if *recorded == obj as usize
        )), "expected ObjSetWidth for left slot width, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::ObjSetHeight { obj: recorded, h: 56 } if *recorded == obj as usize
        )), "expected ObjSetHeight for field height, got: {:?}", calls);
    }

    #[test]
    fn phone_formatter_style_spy_records_padding_border_and_text() {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        let ink = unsafe { lv_color_hex(0x203844) };
        let paper = unsafe { lv_color_hex(0xFFFFFF) };

        unsafe {
            lv_obj_set_style_pad_left(obj, 14, 0);
            lv_obj_set_style_pad_right(obj, 10, 0);
            lv_obj_set_style_bg_color(obj, paper, 0);
            lv_obj_set_style_bg_opa(obj, 255, 0);
            lv_obj_set_style_text_color(obj, ink, 0);
            lv_obj_set_style_text_opa(obj, 220, 0);
            lv_obj_set_style_border_color(obj, ink, 0);
            lv_obj_set_style_border_width(obj, 2, 0);
            lv_obj_set_style_border_opa(obj, 255, 0);
        }

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStylePadLeft { obj: recorded, value: 14 } if *recorded == obj as usize
        )), "expected SetStylePadLeft, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStylePadRight { obj: recorded, value: 10 } if *recorded == obj as usize
        )), "expected SetStylePadRight, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBgColor { obj: recorded, color } if *recorded == obj as usize && *color == paper
        )), "expected SetStyleBgColor, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBgOpa { obj: recorded, opa: 255 } if *recorded == obj as usize
        )), "expected SetStyleBgOpa, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleTextColor { obj: recorded, color } if *recorded == obj as usize && *color == ink
        )), "expected SetStyleTextColor, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleTextOpa { obj: recorded, opa: 220 } if *recorded == obj as usize
        )), "expected SetStyleTextOpa, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBorderColor { obj: recorded, color } if *recorded == obj as usize && *color == ink
        )), "expected SetStyleBorderColor, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBorderWidth { obj: recorded, value: 2 } if *recorded == obj as usize
        )), "expected SetStyleBorderWidth, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBorderOpa { obj: recorded, opa: 255 } if *recorded == obj as usize
        )), "expected SetStyleBorderOpa, got: {:?}", calls);
    }
```

- [ ] **Step 2: Run the failing c_bindings tests**

Run:

```bash
cargo test phone_formatter_ --quiet
```

Expected: FAIL at compile time with missing `LvCall` variants such as `ObjSetWidth`, `ObjSetHeight`, `SetStylePadLeft`, `SetStylePadRight`, `SetStyleBgColor`, `SetStyleBgOpa`, `SetStyleTextColor`, `SetStyleTextOpa`, `SetStyleBorderColor`, `SetStyleBorderWidth`, and `SetStyleBorderOpa`.

- [ ] **Step 3: Add spy variants**

In `src/c_bindings.rs`, add these variants to the mock `LvCall` enum near `ObjSetSize` and the existing style variants:

```rust
        ObjSetWidth      { obj: usize, w: i32 },
        ObjSetHeight     { obj: usize, h: i32 },
        SetStyleBgColor      { obj: usize, color: lv_color_t },
        SetStyleBgOpa        { obj: usize, opa: u8 },
        SetStyleTextColor    { obj: usize, color: lv_color_t },
        SetStyleTextOpa      { obj: usize, opa: u8 },
        SetStylePadTop       { obj: usize, value: i32 },
        SetStylePadBottom    { obj: usize, value: i32 },
        SetStylePadLeft      { obj: usize, value: i32 },
        SetStylePadRight     { obj: usize, value: i32 },
        SetStyleBorderColor  { obj: usize, color: lv_color_t },
        SetStyleBorderWidth  { obj: usize, value: i32 },
        SetStyleBorderOpa    { obj: usize, opa: u8 },
        SetStyleOpa          { obj: usize, opa: u8 },
```

- [ ] **Step 4: Record mock geometry calls**

Replace the mock width and height no-ops with:

```rust
    pub unsafe fn lv_obj_set_width(obj: *mut lv_obj_t, w: i32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetWidth {
            obj: obj as usize,
            w,
        }));
    }

    pub unsafe fn lv_obj_set_height(obj: *mut lv_obj_t, h: i32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetHeight {
            obj: obj as usize,
            h,
        }));
    }
```

- [ ] **Step 5: Record mock style calls**

Replace the mock no-op style functions from `lv_obj_set_style_pad_top` through `lv_obj_set_style_border_opa` with:

```rust
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
    pub unsafe fn lv_obj_set_style_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleOpa { obj: obj as usize, opa }));
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

Leave `lv_obj_set_style_pad_row`, `lv_obj_set_style_pad_column`, `lv_obj_set_style_border_side`, and outline/shadow setters in their current form unless a later task adds a failing assertion for them.

- [ ] **Step 6: Run the c_bindings tests**

Run:

```bash
cargo test phone_formatter_ --quiet
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/c_bindings.rs
git commit -m "test: add phone formatter spy support" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 2: Add `FormatPreset` formatting and normalization

**Files:**
- Create: `src/lvgl/phone_formatter_field.rs`
- Modify: `src/lvgl/mod.rs`

- [ ] **Step 1: Write failing formatter tests**

Create `src/lvgl/phone_formatter_field.rs` with this initial content:

```rust
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[cfg(test)]
mod tests {
    use super::FormatPreset;

    #[test]
    fn groups_preset_formats_full_phone_number() {
        let preset = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        assert_eq!(preset.format_digits("866029371"), "+41 86 602 93 71");
    }

    #[test]
    fn groups_preset_formats_partial_phone_number() {
        let preset = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        assert_eq!(preset.format_digits("8"), "+41 8");
        assert_eq!(preset.format_digits("8660"), "+41 86 60");
    }

    #[test]
    fn mask_preset_formats_simple_code() {
        let preset = FormatPreset::mask("WECHIP - X X X X X X");
        assert_eq!(preset.format_digits("234567"), "WECHIP - 2 3 4 5 6 7");
        assert_eq!(preset.format_digits("234"), "WECHIP - 2 3 4");
    }

    #[test]
    fn empty_raw_digits_render_empty_text() {
        let phone = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        let code = FormatPreset::mask("WECHIP - X X X X X X");
        assert_eq!(phone.format_digits(""), "");
        assert_eq!(code.format_digits(""), "");
    }

    #[test]
    fn normalize_digits_strips_non_digits_and_truncates_to_capacity() {
        let preset = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        assert_eq!(preset.normalize_digits("+41 86-602-93-7100"), "866029371");
    }

    #[test]
    fn normalize_digits_keeps_plain_digits_when_prefix_is_absent() {
        let preset = FormatPreset::groups("+41 ", &[2, 3, 2, 2]);
        assert_eq!(preset.normalize_digits("86-602-93-71"), "866029371");
    }

    #[test]
    #[should_panic(expected = "FormatPreset::groups requires at least one group")]
    fn groups_preset_rejects_no_groups() {
        let _ = FormatPreset::groups("+41 ", &[]);
    }

    #[test]
    #[should_panic(expected = "FormatPreset::groups requires every group size to be greater than zero")]
    fn groups_preset_rejects_zero_group_size() {
        let _ = FormatPreset::groups("+41 ", &[2, 0, 2]);
    }

    #[test]
    #[should_panic(expected = "FormatPreset::mask requires at least one X digit slot")]
    fn mask_preset_rejects_masks_without_digit_slots() {
        let _ = FormatPreset::mask("WECHIP - ");
    }
}
```

In `src/lvgl/mod.rs`, add the private module declaration near the other widget modules:

```rust
mod phone_formatter_field;
```

- [ ] **Step 2: Run the failing formatter tests**

Run:

```bash
cargo test phone_formatter_field::tests:: --quiet
```

Expected: FAIL at compile time because `FormatPreset` is not defined.

- [ ] **Step 3: Add `FormatPreset` implementation**

Add this code above the test module in `src/lvgl/phone_formatter_field.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatPreset {
    Groups { prefix: String, groups: Vec<usize> },
    Mask { mask: String },
}

impl FormatPreset {
    pub fn groups(prefix: &str, groups: &[usize]) -> Self {
        if groups.is_empty() {
            panic!("FormatPreset::groups requires at least one group");
        }
        if groups.iter().any(|group| *group == 0) {
            panic!("FormatPreset::groups requires every group size to be greater than zero");
        }
        Self::Groups {
            prefix: prefix.to_string(),
            groups: groups.to_vec(),
        }
    }

    pub fn mask(mask: &str) -> Self {
        if !mask.chars().any(|ch| ch == 'X') {
            panic!("FormatPreset::mask requires at least one X digit slot");
        }
        Self::Mask { mask: mask.to_string() }
    }

    pub fn capacity(&self) -> usize {
        match self {
            Self::Groups { groups, .. } => groups.iter().copied().sum(),
            Self::Mask { mask } => mask.chars().filter(|ch| *ch == 'X').count(),
        }
    }

    pub fn max_formatted_len(&self) -> usize {
        match self {
            Self::Groups { prefix, groups } => {
                let digits: usize = groups.iter().copied().sum();
                let separators = groups.len().saturating_sub(1);
                prefix.len() + digits + separators
            }
            Self::Mask { mask } => mask.len(),
        }
    }

    pub fn normalize_digits(&self, input: &str) -> String {
        let source = match self {
            Self::Groups { prefix, .. } => input.strip_prefix(prefix).unwrap_or(input),
            Self::Mask { .. } => input,
        };
        source
            .chars()
            .filter(|ch| ch.is_ascii_digit())
            .take(self.capacity())
            .collect()
    }

    pub fn format_digits(&self, digits: &str) -> String {
        let raw = self.normalize_digits(digits);
        if raw.is_empty() {
            return String::new();
        }
        match self {
            Self::Groups { prefix, groups } => format_groups(prefix, groups, &raw),
            Self::Mask { mask } => format_mask(mask, &raw),
        }
    }
}

fn format_groups(prefix: &str, groups: &[usize], raw: &str) -> String {
    let mut out = String::new();
    out.push_str(prefix);
    let mut index = 0;
    for (group_index, group_size) in groups.iter().copied().enumerate() {
        if index >= raw.len() {
            break;
        }
        if group_index > 0 {
            out.push(' ');
        }
        let end = core::cmp::min(index + group_size, raw.len());
        out.push_str(&raw[index..end]);
        index = end;
    }
    out
}

fn format_mask(mask: &str, raw: &str) -> String {
    let mut out = String::new();
    let mut digits = raw.chars();
    for ch in mask.chars() {
        if ch == 'X' {
            let Some(digit) = digits.next() else {
                break;
            };
            out.push(digit);
        } else {
            out.push(ch);
        }
    }
    out
}
```

- [ ] **Step 4: Run the formatter tests**

Run:

```bash
cargo test phone_formatter_field::tests:: --quiet
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lvgl/phone_formatter_field.rs src/lvgl/mod.rs
git commit -m "feat: add phone formatter presets" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Add composite field construction, exports, and value accessors

**Files:**
- Modify: `src/lvgl/phone_formatter_field.rs`
- Modify: `src/lvgl/mod.rs`
- Modify: `src/lvgl/prelude.rs`

- [ ] **Step 1: Write failing composite tests**

Add these tests to `src/lvgl/phone_formatter_field.rs` inside the existing test module:

```rust
    use super::{LeftSlot, PhoneFormatterField};
    use crate::c_bindings::{reset_obj_pool, spy_drain, LvCall};
    use crate::lvgl::screen::Screen;
    use crate::lvgl::{CornerRadius, Size, Widget};

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_creates_root_left_slot_and_text_area() {
        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
        );

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjCreate { .. })), "expected root/slot object creation, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(c, LvCall::TextAreaCreate { .. })), "expected TextAreaCreate, got: {:?}", calls);
        assert_eq!(field.raw_digits(), "");
        assert_eq!(field.formatted_text(), "");
    }

    #[test]
    fn new_configures_one_line_text_area_with_formatted_capacity() {
        let screen = parent();
        let _field = PhoneFormatterField::new(
            &screen,
            FormatPreset::mask("WECHIP - X X X X X X"),
        );

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(c, LvCall::TextAreaSetOneLine { en: true, .. })), "expected one-line text area, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(c, LvCall::TextAreaSetMaxLength { max: 20, .. })), "expected max formatted length for mask, got: {:?}", calls);
    }

    #[test]
    fn set_raw_digits_updates_raw_and_formatted_text_area() {
        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
        );
        spy_drain();

        field.set_raw_digits("86 602 93 71");

        assert_eq!(field.raw_digits(), "866029371");
        assert_eq!(field.formatted_text(), "+41 86 602 93 71");
        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::TextAreaSetText { text, .. } if text == b"+41 86 602 93 71\0"
        )), "expected formatted TextAreaSetText, got: {:?}", calls);
    }

    #[test]
    fn widget_methods_apply_to_root_object() {
        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
        );
        spy_drain();

        field
            .size(Size::Px(340), Size::Px(56))
            .radius(CornerRadius::Px(8))
            .border_width(2);

        let root = field.raw_ptr();
        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::ObjSetSize { obj, w: 340, h: 56 } if *obj == root
        )), "expected root ObjSetSize, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleRadius { obj, value: 8 } if *obj == root
        )), "expected root radius, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBorderWidth { obj, value: 2 } if *obj == root
        )), "expected root border width, got: {:?}", calls);
    }
```

- [ ] **Step 2: Run the failing composite tests**

Run:

```bash
cargo test phone_formatter_field::tests:: --quiet
```

Expected: FAIL at compile time because `PhoneFormatterField` is not defined.

- [ ] **Step 3: Add composite types and constructor**

Add these imports near the top of `src/lvgl/phone_formatter_field.rs`:

```rust
use alloc::boxed::Box;
use core::cell::{Cell, RefCell};

use crate::c_bindings;

use super::event::LvEventCode;
use super::obj::Obj;
use super::size::Size;
use super::textarea::TextArea;
use super::util::to_null_terminated;
use super::widget::{LvObj, Widget};
```

Add this code below the formatter helper functions:

```rust
struct FieldContext {
    preset: FormatPreset,
    raw_digits: RefCell<String>,
    text_area_ptr: Cell<usize>,
    suppress_event: Cell<bool>,
}

pub struct PhoneFormatterField {
    root: Obj,
    left_slot: Obj,
    text_area: TextArea,
    context: Box<FieldContext>,
}

impl Widget for PhoneFormatterField {
    fn lv_obj(&self) -> &LvObj {
        self.root.lv_obj()
    }
}

impl PhoneFormatterField {
    pub fn new(parent: &impl Widget, preset: FormatPreset) -> Self {
        let max_len = preset.max_formatted_len() as u32;
        let root = Obj::new(parent);
        root.flex_row().set_scrollable(false);

        let left_slot = Obj::new(&root);
        left_slot.set_hidden(true).set_scrollable(false);

        let text_area = TextArea::new(&root);
        text_area
            .one_line(true)
            .max_length(max_len)
            .set_text("")
            .set_scrollable(false)
            .border_width(0)
            .bg_opa(0);

        let context = Box::new(FieldContext {
            preset,
            raw_digits: RefCell::new(String::new()),
            text_area_ptr: Cell::new(text_area.raw_ptr()),
            suppress_event: Cell::new(false),
        });

        let field = Self {
            root,
            left_slot,
            text_area,
            context,
        };
        field.register_value_changed();
        field
    }

    pub fn placeholder_text(&self, text: &str) -> &Self {
        self.text_area.placeholder_text(text);
        self
    }

    pub fn set_raw_digits(&self, input: &str) -> &Self {
        let raw = self.context.preset.normalize_digits(input);
        self.context.raw_digits.replace(raw.clone());
        self.write_display_text(&self.context.preset.format_digits(&raw));
        self
    }

    pub fn raw_digits(&self) -> String {
        self.context.raw_digits.borrow().clone()
    }

    pub fn formatted_text(&self) -> String {
        self.context.preset.format_digits(&self.context.raw_digits.borrow())
    }

    fn write_display_text(&self, text: &str) {
        self.context.suppress_event.set(true);
        self.text_area.set_text(text);
        self.context.suppress_event.set(false);
    }

    fn register_value_changed(&self) {
        unsafe {
            c_bindings::lv_obj_add_event_cb(
                self.text_area.lv_obj().raw(),
                Some(on_textarea_value_changed),
                LvEventCode::ValueChanged as u32,
                self.context.as_ref() as *const FieldContext as *mut core::ffi::c_void,
            );
        }
    }

    #[cfg(test)]
    fn input_raw_ptr(&self) -> usize {
        self.text_area.raw_ptr()
    }
}

impl Drop for PhoneFormatterField {
    fn drop(&mut self) {
        unsafe {
            c_bindings::lv_obj_remove_event_cb_with_user_data(
                self.text_area.lv_obj().raw(),
                Some(on_textarea_value_changed),
                self.context.as_ref() as *const FieldContext as *mut core::ffi::c_void,
            );
        }
    }
}

unsafe extern "C" fn on_textarea_value_changed(e: *mut c_bindings::lv_event_t) {
    unsafe {
        let user_data = c_bindings::lv_event_get_user_data(e);
        if user_data.is_null() {
            return;
        }
        let context = &*(user_data as *const FieldContext);
        if context.suppress_event.get() {
            return;
        }
        let text_area_ptr = context.text_area_ptr.get();
        if text_area_ptr == 0 {
            return;
        }
        let input = TextArea::text_from_raw_ptr(text_area_ptr);
        let raw = context.preset.normalize_digits(&input);
        let formatted = context.preset.format_digits(&raw);
        context.raw_digits.replace(raw);
        if input != formatted {
            let c_string = to_null_terminated(&formatted);
            context.suppress_event.set(true);
            c_bindings::lv_textarea_set_text(
                text_area_ptr as *mut c_bindings::lv_obj_t,
                c_string.as_ptr() as *const core::ffi::c_char,
            );
            context.suppress_event.set(false);
        }
    }
}
```

- [ ] **Step 4: Export the public types**

In `src/lvgl/mod.rs`, add:

```rust
pub use self::phone_formatter_field::{
    FormatPreset, LeftSlot, LeftSlotHandle, PhoneFormatterField,
};
```

near the other `pub use` statements. `LeftSlot` and `LeftSlotHandle` will be defined in Task 5.

In `src/lvgl/prelude.rs`, add:

```rust
pub use super::phone_formatter_field::{
    FormatPreset, LeftSlot, LeftSlotHandle, PhoneFormatterField,
};
```

- [ ] **Step 5: Add temporary left-slot type stubs for exports**

Add these public stubs below `PhoneFormatterField` in `src/lvgl/phone_formatter_field.rs`; Task 5 replaces them with the full API:

```rust
pub struct LeftSlot;

pub struct LeftSlotHandle {
    obj: LvObj,
}

impl Widget for LeftSlotHandle {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}
```

- [ ] **Step 6: Run composite tests**

Run:

```bash
cargo test phone_formatter_field::tests:: --quiet
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/lvgl/phone_formatter_field.rs src/lvgl/mod.rs src/lvgl/prelude.rs
git commit -m "feat: add phone formatter field shell" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 4: Verify live value-changed formatting

**Files:**
- Modify: `src/lvgl/phone_formatter_field.rs`

- [ ] **Step 1: Write failing live-formatting tests**

Add these tests to `src/lvgl/phone_formatter_field.rs` inside the existing test module:

```rust
    #[test]
    fn value_changed_normalizes_user_input_and_rewrites_formatted_text() {
        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
        );
        let input = field.input_raw_ptr() as *mut crate::c_bindings::lv_obj_t;
        spy_drain();

        unsafe {
            crate::c_bindings::lv_textarea_set_text(input, c"+41 86 602 93 7100".as_ptr());
            crate::c_bindings::spy_emit_event(input, crate::c_bindings::LV_EVENT_VALUE_CHANGED);
        }

        assert_eq!(field.raw_digits(), "866029371");
        assert_eq!(field.formatted_text(), "+41 86 602 93 71");
        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::TextAreaSetText { text, .. } if text == b"+41 86 602 93 71\0"
        )), "expected rewritten formatted text, got: {:?}", calls);
    }

    #[test]
    fn value_changed_formats_mask_input_progressively() {
        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::mask("WECHIP - X X X X X X"),
        );
        let input = field.input_raw_ptr() as *mut crate::c_bindings::lv_obj_t;
        spy_drain();

        unsafe {
            crate::c_bindings::lv_textarea_set_text(input, c"abc234".as_ptr());
            crate::c_bindings::spy_emit_event(input, crate::c_bindings::LV_EVENT_VALUE_CHANGED);
        }

        assert_eq!(field.raw_digits(), "234");
        assert_eq!(field.formatted_text(), "WECHIP - 2 3 4");
    }
```

- [ ] **Step 2: Run the live-formatting tests**

Run:

```bash
cargo test value_changed_ --quiet
```

Expected: PASS if Task 3 registered the value-changed callback correctly. If this fails, inspect whether `lv_obj_add_event_cb` was called with `LvEventCode::ValueChanged as u32` and whether `TextArea::text_from_raw_ptr` reads the same text-area pointer stored in `FieldContext`.

- [ ] **Step 3: Add explicit callback registration assertion**

Add this test to keep the event registration visible in the spy log:

```rust
    #[test]
    fn new_registers_value_changed_callback_on_text_area() {
        let screen = parent();
        let _field = PhoneFormatterField::new(
            &screen,
            FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
        );

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::AddEventCb { code, .. } if *code == crate::c_bindings::LV_EVENT_VALUE_CHANGED
        )), "expected value-changed event registration, got: {:?}", calls);
    }
```

- [ ] **Step 4: Run phone formatter tests**

Run:

```bash
cargo test phone_formatter_field::tests:: --quiet
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lvgl/phone_formatter_field.rs
git commit -m "feat: format phone field edits live" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 5: Add left-slot presets and custom-content handle

**Files:**
- Modify: `src/lvgl/phone_formatter_field.rs`

- [ ] **Step 1: Write failing left-slot tests**

Add these tests to `src/lvgl/phone_formatter_field.rs` inside the existing test module:

```rust
    #[test]
    fn left_slot_preset_unhides_slot_and_creates_text_and_arrow_labels() {
        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
        );
        spy_drain();

        field.left_slot(
            LeftSlot::preset()
                .text("CH")
                .arrow("v")
                .width(Size::Px(72))
                .divider(true),
        );

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(c, LvCall::RemoveFlag { .. })), "expected hidden flag removal for left slot, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetWidth { w: 72, .. })), "expected left slot width, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"CH\0"
        )), "expected text label, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"v\0"
        )), "expected arrow label, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(c, LvCall::SetStyleBorderWidth { value: 1, .. })), "expected divider border width, got: {:?}", calls);
    }

    #[test]
    fn left_slot_preset_registers_click_callback() {
        fn on_left_slot(_: crate::lvgl::event::Event) {}

        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
        );
        spy_drain();

        field.left_slot(LeftSlot::preset().text("CH").on_click(on_left_slot));

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::AddEventCb { code, .. } if *code == crate::c_bindings::LV_EVENT_CLICKED
        )), "expected click callback registration, got: {:?}", calls);
    }

    #[test]
    fn custom_left_slot_returns_widget_handle_for_caller_children() {
        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::mask("WECHIP - X X X X X X"),
        );
        spy_drain();

        let slot = field.custom_left_slot(Size::Px(96));
        let _label = crate::lvgl::Label::new(&slot).text("WECHIP");

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetWidth { w: 96, .. })), "expected custom slot width, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"WECHIP\0"
        )), "expected caller child label, got: {:?}", calls);
    }

    #[test]
    #[should_panic(expected = "LeftSlot preset requires text, icon, arrow, or custom content")]
    fn empty_left_slot_preset_panics() {
        let screen = parent();
        let field = PhoneFormatterField::new(
            &screen,
            FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
        );
        field.left_slot(LeftSlot::preset());
    }
```

- [ ] **Step 2: Run the failing left-slot tests**

Run:

```bash
cargo test left_slot_ custom_left_slot empty_left_slot --quiet
```

Expected: FAIL because the temporary `LeftSlot` stub has no builder methods and `PhoneFormatterField` has no left-slot APIs.

- [ ] **Step 3: Replace the left-slot stubs with full types**

Replace the temporary `LeftSlot` and `LeftSlotHandle` stubs with:

```rust
use super::event::Event;
use super::image::{Image, ImageSrc};
use super::label::Label;

pub struct LeftSlot {
    text: Option<String>,
    arrow: Option<String>,
    icon: Option<ImageSrc>,
    width: Option<Size>,
    divider: bool,
    pad_x: i32,
    on_click: Option<fn(Event)>,
}

impl LeftSlot {
    pub fn preset() -> Self {
        Self {
            text: None,
            arrow: None,
            icon: None,
            width: None,
            divider: false,
            pad_x: 10,
            on_click: None,
        }
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    pub fn arrow(mut self, arrow: &str) -> Self {
        self.arrow = Some(arrow.to_string());
        self
    }

    pub fn icon(mut self, icon: ImageSrc) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    pub fn divider(mut self, enabled: bool) -> Self {
        self.divider = enabled;
        self
    }

    pub fn padding(mut self, px: i32) -> Self {
        self.pad_x = px;
        self
    }

    pub fn on_click(mut self, cb: fn(Event)) -> Self {
        self.on_click = Some(cb);
        self
    }

    fn has_visible_content(&self) -> bool {
        self.text.is_some() || self.arrow.is_some() || self.icon.is_some()
    }
}

pub struct LeftSlotHandle {
    obj: LvObj,
}

impl Widget for LeftSlotHandle {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}
```

- [ ] **Step 4: Add left-slot APIs to `PhoneFormatterField`**

Add these methods inside `impl PhoneFormatterField`:

```rust
    pub fn left_slot(&self, slot: LeftSlot) -> &Self {
        if !slot.has_visible_content() {
            panic!("LeftSlot preset requires text, icon, arrow, or custom content");
        }
        self.show_left_slot(slot.width);
        self.left_slot.pad_left(slot.pad_x).pad_right(slot.pad_x);
        if slot.divider {
            self.left_slot.border_width(1);
        }
        if let Some(icon) = slot.icon {
            let _ = Image::new(&self.left_slot).set_src(&icon);
        }
        if let Some(text) = slot.text {
            let _ = Label::new(&self.left_slot).text(&text);
        }
        if let Some(arrow) = slot.arrow {
            let _ = Label::new(&self.left_slot).text(&arrow);
        }
        if let Some(cb) = slot.on_click {
            self.left_slot.set_clickable(true).on_click(cb);
        }
        self
    }

    pub fn custom_left_slot(&self, width: Size) -> LeftSlotHandle {
        self.show_left_slot(Some(width));
        LeftSlotHandle {
            obj: LvObj::from_raw(self.left_slot.lv_obj().raw()),
        }
    }

    fn show_left_slot(&self, width: Option<Size>) {
        self.left_slot.set_hidden(false).set_scrollable(false).flex_row();
        if let Some(width) = width {
            self.left_slot.width(width);
        }
    }
```

- [ ] **Step 5: Run left-slot tests**

Run:

```bash
cargo test left_slot_ custom_left_slot empty_left_slot --quiet
```

Expected: PASS.

- [ ] **Step 6: Run all phone formatter tests**

Run:

```bash
cargo test phone_formatter_field::tests:: --quiet
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/lvgl/phone_formatter_field.rs
git commit -m "feat: add phone formatter left slot" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 6: Document the Rust API

**Files:**
- Modify: `DSL_REFERENCE.md`

- [ ] **Step 1: Add `PhoneFormatterField` to the table of contents**

In `DSL_REFERENCE.md`, add this entry in the widget list immediately after `TextArea`:

```markdown
    - [PhoneFormatterField](#phoneformatterfield)
```

- [ ] **Step 2: Add the reference section**

Insert this section immediately after the existing `### TextArea` section:

```markdown
### PhoneFormatterField

A composite one-line formatted input field for phone numbers and simple numeric
codes.  It stores raw digits only and renders generated prefixes, spaces, and
mask literals in the visible text area.

```rust
let phone = PhoneFormatterField::new(
    &screen,
    FormatPreset::groups("+41 ", &[2, 3, 2, 2]),
)
.left_slot(
    LeftSlot::preset()
        .text("🇨🇭")
        .arrow("▾")
        .width(Size::Px(72))
        .divider(true),
)
.placeholder_text("+41 XX XXX XX XX")
.size(Size::Px(340), Size::Px(56))
.radius(CornerRadius::Px(8))
.border_width(2);

phone.set_raw_digits("866029371");
assert_eq!(phone.raw_digits(), "866029371");
assert_eq!(phone.formatted_text(), "+41 86 602 93 71");
```

Mask presets use `X` as digit slots and copy every other character as generated
display text:

```rust
let code = PhoneFormatterField::new(
    &screen,
    FormatPreset::mask("WECHIP - X X X X X X"),
)
.placeholder_text("WECHIP - X X X X X X");

code.set_raw_digits("234567");
assert_eq!(code.formatted_text(), "WECHIP - 2 3 4 5 6 7");
```

| Method | Description |
|---|---|
| `PhoneFormatterField::new(parent, preset)` | Creates the composite field with a hidden left slot and one-line text area. |
| `FormatPreset::groups(prefix, groups)` | Formats digits after a fixed prefix with explicit group sizes. |
| `FormatPreset::mask(mask)` | Formats digits into `X` positions and emits all other mask characters as literals. |
| `placeholder_text(text)` | Sets the text-area placeholder shown before raw digits exist. |
| `set_raw_digits(text)` | Stores ASCII digits from `text`, truncates to preset capacity, and updates the formatted display. |
| `raw_digits()` | Returns the stored digit-only value. |
| `formatted_text()` | Returns the current generated display string. |
| `left_slot(LeftSlot::preset())` | Enables the built-in left segment with common text/icon/arrow content. |
| `custom_left_slot(width)` | Enables the left segment and returns a widget handle so callers can add custom children. |

Invalid presets panic with clear messages: group presets require at least one
non-zero group, and masks require at least one `X` digit slot. Runtime typing
does not panic; the field strips non-digits and truncates to preset capacity.
```

- [ ] **Step 3: Update the widget trait summary**

In the "Widget trait" summary near the bottom of `DSL_REFERENCE.md`, add `PhoneFormatterField` to the list of widgets implementing `Widget`:

```markdown
Every widget (`Screen`, `Obj`, `Button`, `ButtonMatrix`, `Label`, `Dropdown`, `QrCode`, `Image`, `ImageButton`, `TextArea`, `PhoneFormatterField`, `Keyboard`) implements the `Widget` trait.
```

- [ ] **Step 4: Check markdown references**

Run:

```bash
rg "PhoneFormatterField|FormatPreset|LeftSlot" DSL_REFERENCE.md -n
```

Expected: output includes the table-of-contents entry, section heading, examples, methods table, and widget trait summary.

- [ ] **Step 5: Commit**

```bash
git add DSL_REFERENCE.md
git commit -m "docs: document phone formatter field" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 7: Add playground support

**Files:**
- Modify: `DSL_PLAYGROUND.html`

- [ ] **Step 1: Add playground state defaults**

Find the JavaScript `state` object in `DSL_PLAYGROUND.html` and add this section:

```javascript
phoneFormatter: {
  mode: 'groups',
  prefix: '+41 ',
  groups: '2,3,2,2',
  mask: 'WECHIP - X X X X X X',
  raw: '866029371',
  placeholder: '+41 XX XXX XX XX',
  leftSlot: true,
  leftText: '🇨🇭',
  leftArrow: '▾',
  leftWidth: 72,
  radius: 10,
  borderWidth: 2,
  borderColor: '#203844',
  textColor: '#203844'
},
```

- [ ] **Step 2: Add quick link and section shell**

Add this quick-link after the existing TextArea or Keyboard link:

```html
<a href="#phoneformatter-playground">PhoneFormatterField</a>
```

Add this playground section near the TextArea/Keyboard sections:

```html
<section class="playground" id="phoneformatter-playground">
  <div class="playground-header">
    <div>
      <h2>PhoneFormatterField</h2>
      <p>Composite formatted input with digit-only raw value, generated display text, and optional left slot.</p>
    </div>
    <span class="section-tag">Composite Widget</span>
  </div>
  <div class="playground-grid">
    <div class="preview-card">
      <div class="preview-toolbar">
        <p>Visual approximation — not real LVGL rendering</p>
      </div>
      <div class="preview-content" data-preview="phoneFormatter"></div>
    </div>
    <div class="controls">
      <label>
        <span class="label-row"><span>Preset mode</span></span>
        <select data-section="phoneFormatter" data-key="mode" data-type="string">
          <option value="groups">Prefix + groups</option>
          <option value="mask">Mask with X slots</option>
        </select>
      </label>
      <label>
        <span class="label-row"><span>Prefix</span></span>
        <input type="text" value="+41 " data-section="phoneFormatter" data-key="prefix" data-type="string">
      </label>
      <label>
        <span class="label-row"><span>Groups</span></span>
        <input type="text" value="2,3,2,2" data-section="phoneFormatter" data-key="groups" data-type="string">
      </label>
      <label>
        <span class="label-row"><span>Mask</span></span>
        <input type="text" value="WECHIP - X X X X X X" data-section="phoneFormatter" data-key="mask" data-type="string">
      </label>
      <label>
        <span class="label-row"><span>Raw input</span></span>
        <input type="text" value="866029371" data-section="phoneFormatter" data-key="raw" data-type="string">
      </label>
      <label>
        <span class="label-row"><span>Placeholder</span></span>
        <input type="text" value="+41 XX XXX XX XX" data-section="phoneFormatter" data-key="placeholder" data-type="string">
      </label>
      <label class="toggle-pill">
        <input type="checkbox" checked data-section="phoneFormatter" data-key="leftSlot" data-type="boolean"> Built-in left slot
      </label>
      <label>
        <span class="label-row"><span>Left text</span></span>
        <input type="text" value="🇨🇭" data-section="phoneFormatter" data-key="leftText" data-type="string">
      </label>
      <label>
        <span class="label-row"><span>Left arrow</span></span>
        <input type="text" value="▾" data-section="phoneFormatter" data-key="leftArrow" data-type="string">
      </label>
    </div>
  </div>
  <div class="code-title"><span>Generated DSL</span><span>Formatted input</span></div>
  <pre><code data-code="phoneFormatter"></code></pre>
</section>
```

- [ ] **Step 3: Add preview CSS**

Add this CSS near the other widget preview styles:

```css
.phoneformatter-widget {
  display: inline-flex;
  align-items: center;
  min-width: 360px;
  min-height: 58px;
  overflow: hidden;
  background: #ffffff;
  font-weight: 700;
}
.phoneformatter-left {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  align-self: stretch;
  padding: 0 14px;
  border-right: 2px solid currentColor;
}
.phoneformatter-value {
  flex: 1;
  padding: 0 24px;
  letter-spacing: 0.06em;
}
.phoneformatter-placeholder {
  color: #a8adb3;
}
```

- [ ] **Step 4: Add formatter helpers and renderers**

Add these functions near the other JavaScript render helpers:

```javascript
function phoneFormatterDigits(input, capacity) {
  return String(input || '').replace(/\D/g, '').slice(0, capacity);
}

function phoneFormatterGroups(groupsText) {
  return String(groupsText || '')
    .split(',')
    .map(part => Number(part.trim()))
    .filter(n => Number.isFinite(n) && n > 0);
}

function phoneFormatterCapacity(cfg) {
  if (cfg.mode === 'mask') return String(cfg.mask || '').split('').filter(ch => ch === 'X').length;
  return phoneFormatterGroups(cfg.groups).reduce((sum, n) => sum + n, 0);
}

function phoneFormatterFormat(cfg) {
  const raw = phoneFormatterDigits(cfg.raw, phoneFormatterCapacity(cfg));
  if (!raw) return '';
  if (cfg.mode === 'mask') {
    let out = '';
    let index = 0;
    for (const ch of String(cfg.mask || '')) {
      if (ch === 'X') {
        if (index >= raw.length) break;
        out += raw[index++];
      } else {
        out += ch;
      }
    }
    return out;
  }
  const groups = phoneFormatterGroups(cfg.groups);
  let out = String(cfg.prefix || '');
  let index = 0;
  groups.forEach((size, groupIndex) => {
    if (index >= raw.length) return;
    if (groupIndex > 0) out += ' ';
    out += raw.slice(index, index + size);
    index += size;
  });
  return out;
}

function renderPhoneFormatterPreview() {
  const cfg = state.phoneFormatter;
  const formatted = phoneFormatterFormat(cfg);
  const display = formatted || cfg.placeholder || '';
  const preview = document.querySelector('[data-preview="phoneFormatter"]');
  if (!preview) return;
  preview.innerHTML = `
    <div class="phoneformatter-widget" style="border:${cfg.borderWidth}px solid ${cfg.borderColor};border-radius:${cfg.radius}px;color:${cfg.textColor}">
      ${cfg.leftSlot ? `<div class="phoneformatter-left" style="width:${cfg.leftWidth}px"><span>${escapeHtml(cfg.leftText)}</span><span>${escapeHtml(cfg.leftArrow)}</span></div>` : ''}
      <div class="phoneformatter-value ${formatted ? '' : 'phoneformatter-placeholder'}">${escapeHtml(display)}</div>
    </div>
  `;
}

function codePhoneFormatter() {
  const cfg = state.phoneFormatter;
  const preset = cfg.mode === 'mask'
    ? `FormatPreset::mask("${escapeRust(cfg.mask)}")`
    : `FormatPreset::groups("${escapeRust(cfg.prefix)}", &[${phoneFormatterGroups(cfg.groups).join(', ')}])`;
  const leftSlot = cfg.leftSlot
    ? `
    .left_slot(
        LeftSlot::preset()
            .text("${escapeRust(cfg.leftText)}")
            .arrow("${escapeRust(cfg.leftArrow)}")
            .width(Size::Px(${cfg.leftWidth}))
            .divider(true),
    )`
    : '';
  return `let field = PhoneFormatterField::new(&screen, ${preset})${leftSlot}
    .placeholder_text("${escapeRust(cfg.placeholder)}")
    .radius(CornerRadius::Px(${cfg.radius}))
    .border_width(${cfg.borderWidth})
    .text_color(Color::hex(${hexToRust(cfg.textColor)}));

field.set_raw_digits("${escapeRust(cfg.raw)}");`;
}
```

If `escapeHtml`, `escapeRust`, or `hexToRust` do not exist, add these helpers once near the existing JavaScript utility functions:

```javascript
function escapeHtml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function escapeRust(value) {
  return String(value).replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

function hexToRust(value) {
  return '0x' + String(value || '#000000').replace('#', '').toUpperCase();
}
```

- [ ] **Step 5: Wire renderer and code generator**

Find the central render/update function that calls widget preview and code generators. Add calls equivalent to the existing sections:

```javascript
renderPhoneFormatterPreview();
setCode('phoneFormatter', codePhoneFormatter());
```

Use the existing helper name for setting code blocks. If the file uses direct DOM assignment instead of `setCode`, use this exact assignment:

```javascript
const phoneFormatterCode = document.querySelector('[data-code="phoneFormatter"]');
if (phoneFormatterCode) phoneFormatterCode.textContent = codePhoneFormatter();
```

- [ ] **Step 6: Verify playground references**

Run:

```bash
rg "phoneFormatter|PhoneFormatterField|FormatPreset|LeftSlot" DSL_PLAYGROUND.html -n
```

Expected: output includes state defaults, quick link, section markup, CSS class names, formatter helpers, preview renderer, and code generator.

- [ ] **Step 7: Commit**

```bash
git add DSL_PLAYGROUND.html
git commit -m "docs: add phone formatter playground" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 8: Run full verification

**Files:**
- Verify: repository test/build/doc state

- [ ] **Step 1: Run all tests**

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

Expected: PASS. If it fails, run `cargo fmt`, inspect `git --no-pager diff`, and rerun `cargo fmt --check`.

- [ ] **Step 3: Check final diff**

Run:

```bash
git --no-pager diff --stat HEAD~7..HEAD
git --no-pager status --short
```

Expected: the diff includes `src/lvgl/phone_formatter_field.rs`, `src/lvgl/mod.rs`, `src/lvgl/prelude.rs`, `src/c_bindings.rs`, `DSL_REFERENCE.md`, and `DSL_PLAYGROUND.html`; status is clean.

- [ ] **Step 4: Commit any formatting-only changes**

If `cargo fmt` changed files after the previous task commits, commit them:

```bash
git add src/lvgl/phone_formatter_field.rs src/lvgl/mod.rs src/lvgl/prelude.rs src/c_bindings.rs
git commit -m "style: format phone formatter field" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

Expected: commit is created only when `git --no-pager diff --quiet` reports tracked formatting changes.
