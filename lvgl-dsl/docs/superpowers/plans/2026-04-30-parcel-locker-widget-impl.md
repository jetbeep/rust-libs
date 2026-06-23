# ParcelLocker Widget Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an image-backed interactive `ParcelLocker` widget with arbitrary rectangular cell hit areas, caller-defined status styling, single-selection highlighting, and typed tap callbacks.

**Architecture:** Implement `ParcelLocker` as a composite LVGL widget: one root `lv_obj` receives the background image and one child `lv_obj` overlay is created per cell rectangle. Runtime state lives in `Rc<RefCell<...>>` allocations so the Rust wrapper can move safely while LVGL event callbacks hold stable boxed per-cell contexts. Styling resolves default, status, disabled, and selected layers into concrete LVGL style calls for each overlay.

**Tech Stack:** Rust 2024, `no_std` crate with `extern crate alloc`, LVGL v9.2 object/style/event APIs, existing mock `c_bindings` spy layer, Markdown reference docs, standalone HTML/CSS/JavaScript playground.

---

## File Structure

- Create `src/lvgl/parcel_locker.rs`: owns public parcel-locker types, layout validation, composite widget construction, style resolution, state mutation APIs, callback trampoline, drop cleanup, and unit tests.
- Modify `src/lvgl/color.rs`: derive `Copy` and `Clone` for `Color` so styles can be stored and merged cheaply.
- Modify `src/lvgl/mod.rs`: add the `parcel_locker` module and public re-exports.
- Modify `src/lvgl/prelude.rs`: re-export the public parcel-locker API for `use lvgl_dsl::lvgl::prelude::*`.
- Modify `src/c_bindings.rs`: add mock spy variants/functions for object position and style calls needed by overlay geometry and visual tests.
- Modify `DSL_REFERENCE.md`: document the new widget and supporting types.
- Modify `DSL_PLAYGROUND.html`: add a ParcelLocker section, controls, preview renderer, and generated code.

## Implementation Tasks

### Task 1: Add mock spy support for overlay geometry and styles

**Files:**
- Modify: `src/c_bindings.rs`

- [ ] **Step 1: Write failing c_bindings spy tests**

Add these tests inside `src/c_bindings.rs` in `#[cfg(test)] mod tests`, after `task1_event_registry_dispatches`:

```rust
    #[test]
    fn parcel_locker_geometry_spy_records_position() {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        unsafe { lv_obj_set_pos(obj, 12, 34); }
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ObjSetPos { obj: recorded, x: 12, y: 34 }
                    if *recorded == obj as usize
            )),
            "expected ObjSetPos for overlay geometry, got: {:?}",
            calls
        );
    }

    #[test]
    fn parcel_locker_style_spy_records_visual_calls() {
        let _fx = SpyFixture::new();
        let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };
        let green = unsafe { lv_color_hex(0x00AA00) };
        let blue = unsafe { lv_color_hex(0x00AEEF) };

        unsafe {
            lv_obj_set_style_bg_color(obj, green, 0);
            lv_obj_set_style_bg_opa(obj, 80, 0);
            lv_obj_set_style_border_width(obj, 2, 0);
            lv_obj_set_style_outline_color(obj, blue, 0);
            lv_obj_set_style_outline_width(obj, 3, 0);
            lv_obj_set_style_opa(obj, 160, 0);
        }

        let calls = spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBgColor { obj: recorded, color }
                if *recorded == obj as usize && *color == green
        )), "expected SetStyleBgColor, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBgOpa { obj: recorded, opa: 80 }
                if *recorded == obj as usize
        )), "expected SetStyleBgOpa, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleBorderWidth { obj: recorded, value: 2 }
                if *recorded == obj as usize
        )), "expected SetStyleBorderWidth, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleOutlineColor { obj: recorded, color }
                if *recorded == obj as usize && *color == blue
        )), "expected SetStyleOutlineColor, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleOutlineWidth { obj: recorded, value: 3 }
                if *recorded == obj as usize
        )), "expected SetStyleOutlineWidth, got: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            LvCall::SetStyleOpa { obj: recorded, opa: 160 }
                if *recorded == obj as usize
        )), "expected SetStyleOpa, got: {:?}", calls);
    }
```

- [ ] **Step 2: Run the failing c_bindings tests**

Run:

```bash
cargo test parcel_locker_ --quiet
```

Expected: FAIL at compile time with missing `LvCall` variants such as `ObjSetPos`, `SetStyleBgColor`, `SetStyleBgOpa`, `SetStyleBorderWidth`, `SetStyleOutlineColor`, `SetStyleOutlineWidth`, and `SetStyleOpa`.

- [ ] **Step 3: Add spy variants**

In `src/c_bindings.rs`, add these variants to the mock `LvCall` enum near the existing `ObjSetSize` and style variants:

```rust
        ObjSetPos        { obj: usize, x: i32, y: i32 },
        SetStyleBgColor       { obj: usize, color: lv_color_t },
        SetStyleBgOpa         { obj: usize, opa: u8 },
        SetStyleBorderColor   { obj: usize, color: lv_color_t },
        SetStyleBorderWidth   { obj: usize, value: i32 },
        SetStyleBorderOpa     { obj: usize, opa: u8 },
        SetStyleOutlineColor  { obj: usize, color: lv_color_t },
        SetStyleOutlineWidth  { obj: usize, value: i32 },
        SetStyleOutlineOpa    { obj: usize, opa: u8 },
        SetStyleOutlinePad    { obj: usize, value: i32 },
        SetStyleOpa           { obj: usize, opa: u8 },
```

- [ ] **Step 4: Record mock geometry and style calls**

Replace the mock `lv_obj_set_pos` no-op with:

```rust
    pub unsafe fn lv_obj_set_pos(obj: *mut lv_obj_t, x: i32, y: i32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::ObjSetPos {
            obj: obj as usize,
            x,
            y,
        }));
    }
```

Replace the no-op style functions from `lv_obj_set_style_bg_color` through `lv_obj_set_style_outline_pad` with:

```rust
    pub unsafe fn lv_obj_set_style_bg_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleBgColor { obj: obj as usize, color }));
    }
    pub unsafe fn lv_obj_set_style_bg_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleBgOpa { obj: obj as usize, opa }));
    }
    pub unsafe fn lv_obj_set_style_text_color(_: *mut lv_obj_t, _: lv_color_t, _: u32) {}
    pub unsafe fn lv_obj_set_style_text_opa(_: *mut lv_obj_t, _: u8, _: u32) {}
    pub unsafe fn lv_obj_set_style_radius(obj: *mut lv_obj_t, value: i32, _selector: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleRadius { obj: obj as usize, value }));
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
    pub unsafe fn lv_obj_set_style_border_side(_: *mut lv_obj_t, _: u32, _: u32) {}
    pub unsafe fn lv_obj_set_style_outline_color(obj: *mut lv_obj_t, color: lv_color_t, _: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleOutlineColor { obj: obj as usize, color }));
    }
    pub unsafe fn lv_obj_set_style_outline_width(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleOutlineWidth { obj: obj as usize, value }));
    }
    pub unsafe fn lv_obj_set_style_outline_opa(obj: *mut lv_obj_t, opa: u8, _: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleOutlineOpa { obj: obj as usize, opa }));
    }
    pub unsafe fn lv_obj_set_style_outline_pad(obj: *mut lv_obj_t, value: i32, _: u32) {
        SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleOutlinePad { obj: obj as usize, value }));
    }
```

- [ ] **Step 5: Run the c_bindings tests**

Run:

```bash
cargo test parcel_locker_ --quiet
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/c_bindings.rs
git commit -m "test: add parcel locker spy support" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 2: Add ParcelLocker public types and layout validation

**Files:**
- Create: `src/lvgl/parcel_locker.rs`
- Modify: `src/lvgl/color.rs`
- Modify: `src/lvgl/mod.rs`

- [ ] **Step 1: Write failing type and validation tests**

Create `src/lvgl/parcel_locker.rs` with this test module and minimal imports:

```rust
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use crate::c_bindings;

use super::color::Color;
use super::widget::{LvObj, Widget};

#[cfg(test)]
mod tests {
    use super::*;

    static CELLS: &[ParcelLockerCell] = &[
        ParcelLockerCell::new(0, 0, CellRect::new(10, 20, 80, 60)),
        ParcelLockerCell::new(0, 1, CellRect::new(96, 20, 80, 60)),
        ParcelLockerCell::new(1, 0, CellRect::new(10, 86, 80, 120)),
    ];

    #[test]
    fn cell_rect_constructor_stores_geometry() {
        let rect = CellRect::new(10, 20, 80, 60);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.w, 80);
        assert_eq!(rect.h, 60);
    }

    #[test]
    fn parcel_locker_cell_constructor_stores_metadata() {
        let cell = ParcelLockerCell::new(1, 2, CellRect::new(3, 4, 5, 6));
        assert_eq!(cell.row, 1);
        assert_eq!(cell.col, 2);
        assert_eq!(cell.rect, CellRect::new(3, 4, 5, 6));
    }

    #[test]
    fn validate_layout_accepts_unique_cells_inside_matrix() {
        validate_layout(2, 2, CELLS);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker matrix dimensions must be non-zero")]
    fn validate_layout_rejects_zero_dimensions() {
        validate_layout(0, 2, CELLS);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker requires at least one cell")]
    fn validate_layout_rejects_empty_cells() {
        validate_layout(2, 2, &[]);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell 0 rectangle must have positive width and height")]
    fn validate_layout_rejects_non_positive_rectangles() {
        static BAD: &[ParcelLockerCell] = &[
            ParcelLockerCell::new(0, 0, CellRect::new(0, 0, 0, 20)),
        ];
        validate_layout(1, 1, BAD);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell 0 row 2 is outside row count 2")]
    fn validate_layout_rejects_row_outside_matrix() {
        static BAD: &[ParcelLockerCell] = &[
            ParcelLockerCell::new(2, 0, CellRect::new(0, 0, 10, 10)),
        ];
        validate_layout(2, 1, BAD);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell 0 column 3 is outside column count 3")]
    fn validate_layout_rejects_column_outside_matrix() {
        static BAD: &[ParcelLockerCell] = &[
            ParcelLockerCell::new(0, 3, CellRect::new(0, 0, 10, 10)),
        ];
        validate_layout(1, 3, BAD);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker duplicate matrix coordinate row 0 column 0")]
    fn validate_layout_rejects_duplicate_row_column() {
        static BAD: &[ParcelLockerCell] = &[
            ParcelLockerCell::new(0, 0, CellRect::new(0, 0, 10, 10)),
            ParcelLockerCell::new(0, 0, CellRect::new(12, 0, 10, 10)),
        ];
        validate_layout(1, 1, BAD);
    }
}
```

- [ ] **Step 2: Register the private module so tests compile**

In `src/lvgl/mod.rs`, add the module near the other widget modules:

```rust
mod parcel_locker;
```

- [ ] **Step 3: Run the failing validation tests**

Run:

```bash
cargo test lvgl::parcel_locker --quiet
```

Expected: FAIL because `ParcelLockerCell`, `CellRect`, and `validate_layout` are not defined yet.

- [ ] **Step 4: Make `Color` copyable**

Modify `src/lvgl/color.rs` so the struct derives `Copy` and `Clone`:

```rust
#[derive(Copy, Clone)]
pub struct Color {
    inner: c_bindings::lv_color_t,
}
```

- [ ] **Step 5: Add public types and validation**

In `src/lvgl/parcel_locker.rs`, above the tests, add:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellStatusId(pub u16);

impl CellStatusId {
    pub const DEFAULT: CellStatusId = CellStatusId(0);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl CellRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ParcelLockerCell {
    pub row: usize,
    pub col: usize,
    pub rect: CellRect,
}

impl ParcelLockerCell {
    pub const fn new(row: usize, col: usize, rect: CellRect) -> Self {
        Self { row, col, rect }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellTap {
    pub index: usize,
    pub row: usize,
    pub col: usize,
    pub status: CellStatusId,
    pub disabled: bool,
}

pub(crate) fn validate_layout(rows: usize, cols: usize, cells: &[ParcelLockerCell]) {
    assert!(
        rows > 0 && cols > 0,
        "ParcelLocker matrix dimensions must be non-zero"
    );
    assert!(!cells.is_empty(), "ParcelLocker requires at least one cell");

    let mut seen = BTreeSet::new();
    for (index, cell) in cells.iter().enumerate() {
        assert!(
            cell.rect.w > 0 && cell.rect.h > 0,
            "ParcelLocker cell {} rectangle must have positive width and height",
            index
        );
        assert!(
            cell.row < rows,
            "ParcelLocker cell {} row {} is outside row count {}",
            index,
            cell.row,
            rows
        );
        assert!(
            cell.col < cols,
            "ParcelLocker cell {} column {} is outside column count {}",
            index,
            cell.col,
            cols
        );
        assert!(
            seen.insert((cell.row, cell.col)),
            "ParcelLocker duplicate matrix coordinate row {} column {}",
            cell.row,
            cell.col
        );
    }
}
```

- [ ] **Step 6: Run the validation tests**

Run:

```bash
cargo test lvgl::parcel_locker --quiet
```

Expected: PASS for all validation tests.

- [ ] **Step 7: Commit**

```bash
git add src/lvgl/parcel_locker.rs src/lvgl/color.rs src/lvgl/mod.rs
git commit -m "feat: add parcel locker layout types" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Construct root and per-cell overlay objects

**Files:**
- Modify: `src/lvgl/parcel_locker.rs`

- [ ] **Step 1: Write failing construction tests**

Append these tests to the `tests` module in `src/lvgl/parcel_locker.rs`:

```rust
    fn setup() -> crate::lvgl::Screen {
        crate::c_bindings::reset_obj_pool();
        crate::lvgl::Screen::active()
    }

    #[test]
    fn new_creates_root_and_one_overlay_per_cell() {
        let screen = setup();
        let _locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        let calls = crate::c_bindings::spy_drain();
        let creates: Vec<_> = calls.iter().filter_map(|call| match call {
            crate::c_bindings::LvCall::ObjCreate { obj, parent } => Some((*obj, *parent)),
            _ => None,
        }).collect();

        assert_eq!(creates.len(), 4, "expected root plus three overlays: {:?}", calls);
        let root = creates[0].0;
        assert_eq!(creates[1].1, root, "first overlay should be parented to root");
        assert_eq!(creates[2].1, root, "second overlay should be parented to root");
        assert_eq!(creates[3].1, root, "third overlay should be parented to root");
    }

    #[test]
    fn new_positions_and_sizes_each_overlay_from_cell_rects() {
        let screen = setup();
        let _locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        let calls = crate::c_bindings::spy_drain();

        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::ObjSetPos { x: 10, y: 20, .. }
        )), "missing first overlay position: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::ObjSetSize { w: 80, h: 60, .. }
        )), "missing first overlay size: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::ObjSetPos { x: 96, y: 20, .. }
        )), "missing second overlay position: {:?}", calls);
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::ObjSetSize { w: 80, h: 120, .. }
        )), "missing tall overlay size: {:?}", calls);
    }

    #[test]
    fn background_applies_image_to_root() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        crate::c_bindings::spy_drain();
        let dummy: u8 = 0;
        let src = unsafe { crate::lvgl::ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };

        locker.background(&src);

        let calls = crate::c_bindings::spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::SetStyleBgImageSrc { src: recorded, .. }
                if *recorded == core::ptr::addr_of!(dummy) as usize
        )), "expected background image source call, got: {:?}", calls);
    }
```

- [ ] **Step 2: Run the failing construction tests**

Run:

```bash
cargo test lvgl::parcel_locker::tests --quiet
```

Expected: FAIL because `ParcelLocker::new`, `ParcelLocker::background`, and `Widget` implementation are not defined.

- [ ] **Step 3: Add runtime structs and constructor**

Add these imports at the top of `src/lvgl/parcel_locker.rs`:

```rust
use core::cell::RefCell;
use core::ffi::c_void;

use alloc::boxed::Box;
use alloc::rc::Rc;

use super::image::ImageSrc;
use super::state::LvObjFlag;
```

Add these structs and implementations above the tests:

```rust
struct CellRuntime {
    definition: ParcelLockerCell,
    overlay: *mut c_bindings::lv_obj_t,
    status: CellStatusId,
    disabled: bool,
}

struct ParcelLockerInner {
    rows: usize,
    cols: usize,
    cells: Vec<CellRuntime>,
    selected: Option<usize>,
    default_style: CellStyle,
    selected_style: CellStyle,
    disabled_style: CellStyle,
    status_styles: BTreeMap<CellStatusId, CellStyle>,
}

struct CellEventCtx {
    inner: Rc<RefCell<ParcelLockerInner>>,
    callback: Rc<RefCell<Option<Box<dyn FnMut(CellTap)>>>>,
    index: usize,
}

pub struct ParcelLocker {
    root: LvObj,
    inner: Rc<RefCell<ParcelLockerInner>>,
    callback: Rc<RefCell<Option<Box<dyn FnMut(CellTap)>>>>,
    event_contexts: Vec<Box<CellEventCtx>>,
}

impl Widget for ParcelLocker {
    fn lv_obj(&self) -> &LvObj {
        &self.root
    }
}

impl ParcelLocker {
    pub fn new(parent: &impl Widget, rows: usize, cols: usize, cells: &[ParcelLockerCell]) -> Self {
        validate_layout(rows, cols, cells);

        let root = unsafe { c_bindings::lv_obj_create(parent.lv_obj().raw()) };
        if root.is_null() {
            panic!("lv_obj_create returned null for ParcelLocker root");
        }

        let mut runtimes = Vec::with_capacity(cells.len());
        for cell in cells {
            let overlay = unsafe { c_bindings::lv_obj_create(root) };
            if overlay.is_null() {
                panic!("lv_obj_create returned null for ParcelLocker cell overlay");
            }
            unsafe {
                c_bindings::lv_obj_set_pos(overlay, cell.rect.x, cell.rect.y);
                c_bindings::lv_obj_set_size(overlay, cell.rect.w, cell.rect.h);
                c_bindings::lv_obj_add_flag(overlay, LvObjFlag::CLICKABLE.0);
            }
            runtimes.push(CellRuntime {
                definition: *cell,
                overlay,
                status: CellStatusId::DEFAULT,
                disabled: false,
            });
        }

        let inner = Rc::new(RefCell::new(ParcelLockerInner {
            rows,
            cols,
            cells: runtimes,
            selected: None,
            default_style: CellStyle::transparent(),
            selected_style: CellStyle::outline(Color::hex(0x00AEEF), 3),
            disabled_style: CellStyle::opacity(160),
            status_styles: BTreeMap::new(),
        }));

        ParcelLocker {
            root: LvObj::from_raw(root),
            inner,
            callback: Rc::new(RefCell::new(None)),
            event_contexts: Vec::new(),
        }
    }

    pub fn background(&self, src: &ImageSrc) -> &Self {
        unsafe {
            c_bindings::lv_obj_set_style_bg_image_src(self.root.raw(), src.as_ptr(), 0);
        }
        self
    }
}
```

- [ ] **Step 4: Add the first `CellStyle` implementation**

Add this above `CellRuntime`:

```rust
#[derive(Copy, Clone)]
pub struct CellStyle {
    bg_color: Option<Color>,
    bg_opa: Option<u8>,
    border_color: Option<Color>,
    border_width: Option<i32>,
    border_opa: Option<u8>,
    outline_color: Option<Color>,
    outline_width: Option<i32>,
    outline_opa: Option<u8>,
    outline_pad: Option<i32>,
    opa: Option<u8>,
}

impl CellStyle {
    pub const fn transparent() -> Self {
        Self {
            bg_color: None,
            bg_opa: Some(0),
            border_color: None,
            border_width: Some(0),
            border_opa: Some(0),
            outline_color: None,
            outline_width: Some(0),
            outline_opa: Some(0),
            outline_pad: Some(0),
            opa: Some(255),
        }
    }

    pub fn overlay(color: Color, opa: u8) -> Self {
        Self { bg_color: Some(color), bg_opa: Some(opa), ..Self::transparent() }
    }

    pub fn outline(color: Color, width: i32) -> Self {
        Self {
            outline_color: Some(color),
            outline_width: Some(width),
            outline_opa: Some(255),
            outline_pad: Some(0),
            ..Self::transparent()
        }
    }

    pub const fn opacity(opa: u8) -> Self {
        Self { opa: Some(opa), ..Self::transparent() }
    }
}
```

- [ ] **Step 5: Run the construction tests**

Run:

```bash
cargo test lvgl::parcel_locker::tests --quiet
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lvgl/parcel_locker.rs
git commit -m "feat: construct parcel locker overlays" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 4: Implement status, selection, disabled state, and style resolution

**Files:**
- Modify: `src/lvgl/parcel_locker.rs`

- [ ] **Step 1: Write failing state and style tests**

Append these tests to `src/lvgl/parcel_locker.rs`:

```rust
    #[test]
    fn set_status_applies_status_style_to_target_overlay() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.status_style(CellStatusId(7), CellStyle::overlay(Color::hex(0x00AA00), 88));
        crate::c_bindings::spy_drain();

        locker.set_status(1, CellStatusId(7));

        let calls = crate::c_bindings::spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::SetStyleBgOpa { opa: 88, .. }
        )), "expected status style opacity, got: {:?}", calls);
        assert_eq!(locker.cell_status(1), CellStatusId(7));
    }

    #[test]
    fn set_status_without_mapping_falls_back_to_default_style() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.default_style(CellStyle::overlay(Color::hex(0x111111), 22));
        crate::c_bindings::spy_drain();

        locker.set_status(2, CellStatusId(99));

        let calls = crate::c_bindings::spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::SetStyleBgOpa { opa: 22, .. }
        )), "expected default style fallback, got: {:?}", calls);
    }

    #[test]
    fn set_selected_moves_single_selection() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.selected_style(CellStyle::outline(Color::hex(0x00AEEF), 4));
        crate::c_bindings::spy_drain();

        locker.set_selected(Some(0));
        locker.set_selected(Some(2));

        let calls = crate::c_bindings::spy_drain();
        let outline_width_calls = calls.iter().filter(|c| matches!(
            c,
            crate::c_bindings::LvCall::SetStyleOutlineWidth { .. }
        )).count();
        assert!(outline_width_calls >= 2, "expected prior and new selection restyles: {:?}", calls);
        assert_eq!(locker.selected(), Some(2));
    }

    #[test]
    fn set_selected_none_clears_selection() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.set_selected(Some(0));
        crate::c_bindings::spy_drain();

        locker.set_selected(None);

        assert_eq!(locker.selected(), None);
        let calls = crate::c_bindings::spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::SetStyleOutlineWidth { value: 0, .. }
        )), "expected cleared outline width, got: {:?}", calls);
    }

    #[test]
    fn disabled_state_is_queryable_and_restyled() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        crate::c_bindings::spy_drain();

        locker.set_disabled(1, true);

        assert!(locker.is_disabled(1));
        let calls = crate::c_bindings::spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::SetStyleOpa { opa: 160, .. }
        )), "expected disabled opacity style, got: {:?}", calls);
    }

    #[test]
    #[should_panic(expected = "ParcelLocker cell index 99 is out of range 0..3")]
    fn index_methods_panic_on_out_of_range() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.set_status(99, CellStatusId(1));
    }
```

- [ ] **Step 2: Run the failing state tests**

Run:

```bash
cargo test lvgl::parcel_locker::tests --quiet
```

Expected: FAIL because state/style APIs and restyling are not implemented.

- [ ] **Step 3: Add style merge and application helpers**

Add this internal resolved style type and helper functions above `impl ParcelLocker`:

```rust
#[derive(Copy, Clone)]
struct ResolvedCellStyle {
    bg_color: Color,
    bg_opa: u8,
    border_color: Color,
    border_width: i32,
    border_opa: u8,
    outline_color: Color,
    outline_width: i32,
    outline_opa: u8,
    outline_pad: i32,
    opa: u8,
}

impl ResolvedCellStyle {
    fn blank() -> Self {
        Self {
            bg_color: Color::black(),
            bg_opa: 0,
            border_color: Color::black(),
            border_width: 0,
            border_opa: 0,
            outline_color: Color::black(),
            outline_width: 0,
            outline_opa: 0,
            outline_pad: 0,
            opa: 255,
        }
    }

    fn apply_patch(&mut self, patch: CellStyle) {
        if let Some(value) = patch.bg_color { self.bg_color = value; }
        if let Some(value) = patch.bg_opa { self.bg_opa = value; }
        if let Some(value) = patch.border_color { self.border_color = value; }
        if let Some(value) = patch.border_width { self.border_width = value; }
        if let Some(value) = patch.border_opa { self.border_opa = value; }
        if let Some(value) = patch.outline_color { self.outline_color = value; }
        if let Some(value) = patch.outline_width { self.outline_width = value; }
        if let Some(value) = patch.outline_opa { self.outline_opa = value; }
        if let Some(value) = patch.outline_pad { self.outline_pad = value; }
        if let Some(value) = patch.opa { self.opa = value; }
    }
}

fn apply_resolved_style(obj: *mut c_bindings::lv_obj_t, style: ResolvedCellStyle) {
    unsafe {
        c_bindings::lv_obj_set_style_bg_color(obj, style.bg_color.to_lv(), 0);
        c_bindings::lv_obj_set_style_bg_opa(obj, style.bg_opa, 0);
        c_bindings::lv_obj_set_style_border_color(obj, style.border_color.to_lv(), 0);
        c_bindings::lv_obj_set_style_border_width(obj, style.border_width, 0);
        c_bindings::lv_obj_set_style_border_opa(obj, style.border_opa, 0);
        c_bindings::lv_obj_set_style_outline_color(obj, style.outline_color.to_lv(), 0);
        c_bindings::lv_obj_set_style_outline_width(obj, style.outline_width, 0);
        c_bindings::lv_obj_set_style_outline_opa(obj, style.outline_opa, 0);
        c_bindings::lv_obj_set_style_outline_pad(obj, style.outline_pad, 0);
        c_bindings::lv_obj_set_style_opa(obj, style.opa, 0);
    }
}

fn assert_cell_index(index: usize, len: usize) {
    assert!(
        index < len,
        "ParcelLocker cell index {} is out of range 0..{}",
        index,
        len
    );
}
```

- [ ] **Step 4: Add state/style APIs**

Inside `impl ParcelLocker`, after `background`, add:

```rust
    pub fn default_style(&self, style: CellStyle) -> &Self {
        self.inner.borrow_mut().default_style = style;
        self.restyle_all();
        self
    }

    pub fn status_style(&self, status: CellStatusId, style: CellStyle) -> &Self {
        self.inner.borrow_mut().status_styles.insert(status, style);
        self.restyle_all_matching_status(status);
        self
    }

    pub fn selected_style(&self, style: CellStyle) -> &Self {
        self.inner.borrow_mut().selected_style = style;
        if let Some(index) = self.inner.borrow().selected {
            self.restyle_cell(index);
        }
        self
    }

    pub fn disabled_style(&self, style: CellStyle) -> &Self {
        self.inner.borrow_mut().disabled_style = style;
        let len = self.inner.borrow().cells.len();
        for index in 0..len {
            if self.inner.borrow().cells[index].disabled {
                self.restyle_cell(index);
            }
        }
        self
    }

    pub fn set_status(&self, index: usize, status: CellStatusId) -> &Self {
        let len = self.inner.borrow().cells.len();
        assert_cell_index(index, len);
        self.inner.borrow_mut().cells[index].status = status;
        self.restyle_cell(index);
        self
    }

    pub fn cell_status(&self, index: usize) -> CellStatusId {
        let inner = self.inner.borrow();
        assert_cell_index(index, inner.cells.len());
        inner.cells[index].status
    }

    pub fn set_selected(&self, selected: Option<usize>) -> &Self {
        let len = self.inner.borrow().cells.len();
        if let Some(index) = selected {
            assert_cell_index(index, len);
        }

        let previous = self.inner.borrow().selected;
        if previous == selected {
            return self;
        }

        self.inner.borrow_mut().selected = selected;
        if let Some(index) = previous {
            self.restyle_cell(index);
        }
        if let Some(index) = selected {
            self.restyle_cell(index);
        }
        self
    }

    pub fn selected(&self) -> Option<usize> {
        self.inner.borrow().selected
    }

    pub fn clear_selected(&self) -> &Self {
        self.set_selected(None)
    }

    pub fn set_disabled(&self, index: usize, disabled: bool) -> &Self {
        let len = self.inner.borrow().cells.len();
        assert_cell_index(index, len);
        self.inner.borrow_mut().cells[index].disabled = disabled;
        self.restyle_cell(index);
        self
    }

    pub fn is_disabled(&self, index: usize) -> bool {
        let inner = self.inner.borrow();
        assert_cell_index(index, inner.cells.len());
        inner.cells[index].disabled
    }

    fn restyle_all(&self) {
        let len = self.inner.borrow().cells.len();
        for index in 0..len {
            self.restyle_cell(index);
        }
    }

    fn restyle_all_matching_status(&self, status: CellStatusId) {
        let len = self.inner.borrow().cells.len();
        for index in 0..len {
            if self.inner.borrow().cells[index].status == status {
                self.restyle_cell(index);
            }
        }
    }

    fn restyle_cell(&self, index: usize) {
        let (overlay, resolved) = {
            let inner = self.inner.borrow();
            assert_cell_index(index, inner.cells.len());
            let cell = &inner.cells[index];
            let mut resolved = ResolvedCellStyle::blank();
            resolved.apply_patch(inner.default_style);
            if let Some(status_style) = inner.status_styles.get(&cell.status).copied() {
                resolved.apply_patch(status_style);
            }
            if cell.disabled {
                resolved.apply_patch(inner.disabled_style);
            }
            if inner.selected == Some(index) {
                resolved.apply_patch(inner.selected_style);
            }
            (cell.overlay, resolved)
        };

        apply_resolved_style(overlay, resolved);
    }
```

- [ ] **Step 5: Run the state tests**

Run:

```bash
cargo test lvgl::parcel_locker::tests --quiet
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lvgl/parcel_locker.rs
git commit -m "feat: style parcel locker cells" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 5: Add cell tap callback trampoline

**Files:**
- Modify: `src/lvgl/parcel_locker.rs`

- [ ] **Step 1: Write failing callback tests**

Append these tests to `src/lvgl/parcel_locker.rs`:

```rust
    #[test]
    fn cell_tap_callback_reports_index_metadata_status_and_disabled() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.set_status(1, CellStatusId(7)).set_disabled(1, true);

        let captured = alloc::rc::Rc::new(core::cell::RefCell::new(None));
        let captured_for_cb = captured.clone();
        locker.on_cell_tap(move |tap| {
            *captured_for_cb.borrow_mut() = Some(tap);
        });

        let overlay = locker.cell_overlay_raw(1);
        crate::c_bindings::spy_emit_event(
            overlay as *mut crate::c_bindings::lv_obj_t,
            crate::c_bindings::LV_EVENT_CLICKED,
        );

        assert_eq!(
            *captured.borrow(),
            Some(CellTap {
                index: 1,
                row: 0,
                col: 1,
                status: CellStatusId(7),
                disabled: true,
            })
        );
    }

    #[test]
    fn disabled_cells_still_emit_callbacks() {
        let screen = setup();
        let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
        locker.set_disabled(2, true);

        static FIRES: core::sync::atomic::AtomicUsize =
            core::sync::atomic::AtomicUsize::new(0);
        locker.on_cell_tap(|tap| {
            assert_eq!(tap.index, 2);
            assert!(tap.disabled);
            FIRES.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        });

        let overlay = locker.cell_overlay_raw(2);
        crate::c_bindings::spy_emit_event(
            overlay as *mut crate::c_bindings::lv_obj_t,
            crate::c_bindings::LV_EVENT_CLICKED,
        );

        assert_eq!(FIRES.load(core::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn drop_unregisters_cell_event_callbacks() {
        let screen = setup();
        let overlay;
        {
            let locker = ParcelLocker::new(&screen, 2, 2, CELLS);
            locker.on_cell_tap(|_| {});
            overlay = locker.cell_overlay_raw(0);
            crate::c_bindings::spy_drain();
        }

        let calls = crate::c_bindings::spy_drain();
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::c_bindings::LvCall::RemoveEventCbWithUserData { obj, .. }
                if *obj == overlay
        )), "expected event callback cleanup for overlay {overlay:#x}, got: {:?}", calls);
    }
```

- [ ] **Step 2: Run the failing callback tests**

Run:

```bash
cargo test lvgl::parcel_locker::tests --quiet
```

Expected: FAIL because `on_cell_tap`, `cell_overlay_raw`, event registration, and `Drop` cleanup are missing.

- [ ] **Step 3: Add callback registration and test-only overlay accessor**

Inside `impl ParcelLocker`, add:

```rust
    pub fn on_cell_tap(&self, f: impl FnMut(CellTap) + 'static) -> &Self {
        *self.callback.borrow_mut() = Some(Box::new(f));
        self
    }

    #[cfg(test)]
    fn cell_overlay_raw(&self, index: usize) -> usize {
        let inner = self.inner.borrow();
        assert_cell_index(index, inner.cells.len());
        inner.cells[index].overlay as usize
    }
```

- [ ] **Step 4: Register per-cell LVGL callbacks during construction**

In `ParcelLocker::new`, change the returned struct construction to create event contexts before returning:

```rust
        let callback = Rc::new(RefCell::new(None));
        let mut locker = ParcelLocker {
            root: LvObj::from_raw(root),
            inner,
            callback,
            event_contexts: Vec::with_capacity(cells.len()),
        };

        let overlay_count = locker.inner.borrow().cells.len();
        for index in 0..overlay_count {
            let mut ctx = Box::new(CellEventCtx {
                inner: locker.inner.clone(),
                callback: locker.callback.clone(),
                index,
            });
            let overlay = locker.inner.borrow().cells[index].overlay;
            unsafe {
                c_bindings::lv_obj_add_event_cb(
                    overlay,
                    Some(on_cell_clicked),
                    c_bindings::LV_EVENT_CLICKED,
                    ctx.as_mut() as *mut CellEventCtx as *mut c_void,
                );
            }
            locker.event_contexts.push(ctx);
        }

        locker
```

- [ ] **Step 5: Add the trampoline and drop cleanup**

Add this function above `impl ParcelLocker`:

```rust
unsafe extern "C" fn on_cell_clicked(e: *mut c_bindings::lv_event_t) {
    let ctx = unsafe { c_bindings::lv_event_get_user_data(e) } as *mut CellEventCtx;
    if ctx.is_null() {
        return;
    }

    let ctx = unsafe { &mut *ctx };
    let tap = {
        let inner = ctx.inner.borrow();
        assert_cell_index(ctx.index, inner.cells.len());
        let cell = &inner.cells[ctx.index];
        CellTap {
            index: ctx.index,
            row: cell.definition.row,
            col: cell.definition.col,
            status: cell.status,
            disabled: cell.disabled,
        }
    };

    if let Some(callback) = ctx.callback.borrow_mut().as_mut() {
        callback(tap);
    }
}
```

Add this `Drop` implementation below `impl Widget for ParcelLocker`:

```rust
impl Drop for ParcelLocker {
    fn drop(&mut self) {
        for ctx in &mut self.event_contexts {
            let overlay = self.inner.borrow().cells[ctx.index].overlay;
            unsafe {
                c_bindings::lv_obj_remove_event_cb_with_user_data(
                    overlay,
                    Some(on_cell_clicked),
                    ctx.as_mut() as *mut CellEventCtx as *mut c_void,
                );
            }
        }
    }
}
```

- [ ] **Step 6: Run the callback tests**

Run:

```bash
cargo test lvgl::parcel_locker::tests --quiet
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/lvgl/parcel_locker.rs
git commit -m "feat: handle parcel locker cell taps" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 6: Export the public ParcelLocker API

**Files:**
- Modify: `src/lvgl/mod.rs`
- Modify: `src/lvgl/prelude.rs`
- Modify: `src/lvgl/parcel_locker.rs`

- [ ] **Step 1: Write failing export test**

Append this test module to `src/lvgl/parcel_locker.rs`:

```rust
#[cfg(test)]
mod export_tests {
    #[test]
    fn prelude_exports_parcel_locker_types() {
        use crate::lvgl::prelude::*;

        let _status = CellStatusId::DEFAULT;
        let _rect = CellRect::new(0, 0, 10, 10);
        let _cell = ParcelLockerCell::new(0, 0, _rect);
        let _style = CellStyle::transparent();
    }
}
```

- [ ] **Step 2: Run the failing export test**

Run:

```bash
cargo test lvgl::parcel_locker::export_tests::prelude_exports_parcel_locker_types --quiet
```

Expected: FAIL because the public types are not wired into `lvgl::mod` and `lvgl::prelude`.

- [ ] **Step 3: Export the public types**

In `src/lvgl/mod.rs`, add public re-exports near `pub use self::palette::Palette;`:

```rust
pub use self::parcel_locker::{
    CellRect, CellStatusId, CellStyle, CellTap, ParcelLocker, ParcelLockerCell,
};
```

In `src/lvgl/prelude.rs`, add:

```rust
pub use super::parcel_locker::{
    CellRect, CellStatusId, CellStyle, CellTap, ParcelLocker, ParcelLockerCell,
};
```

- [ ] **Step 4: Run parcel locker tests through public exports**

Run:

```bash
cargo test lvgl::parcel_locker --quiet
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lvgl/mod.rs src/lvgl/prelude.rs src/lvgl/parcel_locker.rs
git commit -m "feat: export parcel locker widget" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 7: Update reference documentation

**Files:**
- Modify: `DSL_REFERENCE.md`

- [ ] **Step 1: Add the widget to the table of contents**

In `DSL_REFERENCE.md`, add `ParcelLocker` after `ButtonMatrix` in the widget table of contents:

```markdown
    - [ParcelLocker](#parcellocker)
```

Add supporting type entries after `ButtonMatrixCtrlMap`:

```markdown
    - [CellStatusId](#cellstatusid)
    - [CellRect](#cellrect)
    - [ParcelLockerCell](#parcellockercell)
    - [CellStyle](#cellstyle)
    - [CellTap](#celltap)
```

- [ ] **Step 2: Add the ParcelLocker widget section**

Insert this section after the existing `### ButtonMatrix` section and before `### Label`:

```markdown
### ParcelLocker

An image-backed interactive parcel-locker layout. `ParcelLocker` creates a root LVGL object with a background image and one clickable overlay object per locker cell. Each cell has its own rectangle, so layouts can represent uneven physical locker doors rather than only equal-size grids.

**Construction**

```rust
static LOCKER_CELLS: &[ParcelLockerCell] = &[
    ParcelLockerCell::new(0, 0, CellRect::new(10, 20, 80, 60)),
    ParcelLockerCell::new(0, 1, CellRect::new(96, 20, 80, 60)),
    ParcelLockerCell::new(1, 0, CellRect::new(10, 86, 80, 120)),
];

let bg = ImageSrc::file(c"/lfs/locker.bin");

let locker = ParcelLocker::new(&screen, 2, 2, LOCKER_CELLS)
    .background(&bg)
    .default_style(CellStyle::overlay(Color::hex(0x000000), 40))
    .status_style(CellStatusId(1), CellStyle::overlay(Color::hex(0x00AA00), 80))
    .status_style(CellStatusId(2), CellStyle::overlay(Color::hex(0xFF8800), 80))
    .selected_style(CellStyle::outline(Color::hex(0x00AEEF), 3))
    .size(Size::Px(320), Size::Px(240));
```

**Methods**

| Method | Description |
|--------|-------------|
| `new(parent, rows, cols, cells)` | Creates the root container and one overlay per cell. Panics if dimensions, rectangles, or row/column coordinates are invalid. |
| `background(&ImageSrc)` | Sets the locker background image on the root container. The image source must outlive the widget. |
| `default_style(CellStyle)` | Sets the base style used when a cell's status has no mapping. |
| `status_style(CellStatusId, CellStyle)` | Maps a caller-defined status ID to a visual style. |
| `selected_style(CellStyle)` | Sets the single-selection highlight style. |
| `disabled_style(CellStyle)` | Sets the visual adjustment applied to disabled cells. |
| `set_status(index, CellStatusId)` | Updates one cell's status and restyles that overlay. |
| `cell_status(index)` | Returns one cell's current status. |
| `set_selected(Some(index))` | Selects one cell and clears the previous selection. |
| `set_selected(None)` / `clear_selected()` | Clears selection. |
| `selected()` | Returns the selected cell index, if any. |
| `set_disabled(index, bool)` | Updates the disabled flag and restyles that overlay. Disabled cells still emit callbacks. |
| `is_disabled(index)` | Returns one cell's disabled flag. |
| `on_cell_tap(callback)` | Registers a callback receiving `CellTap { index, row, col, status, disabled }`. |

**Tap handling**

Disabled cells are still clickable. The callback receives `disabled: true`, allowing application logic to show a message, reject the action, or handle a special workflow.
```

- [ ] **Step 3: Add supporting type sections**

In the `## Supporting Types` section, after `### ButtonMatrixCtrlMap`, add:

```markdown
### CellStatusId

Caller-defined parcel-locker cell status identifier. The DSL does not impose business states such as "available" or "occupied"; map your own IDs to styles with `status_style`.

```rust
let available = CellStatusId(1);
let occupied = CellStatusId(2);
locker.status_style(available, CellStyle::overlay(Color::hex(0x00AA00), 80));
locker.status_style(occupied, CellStyle::overlay(Color::hex(0xFF8800), 80));
```

### CellRect

Pixel rectangle for one cell overlay, relative to the parcel-locker root container.

```rust
let rect = CellRect::new(10, 20, 80, 60);
```

### ParcelLockerCell

Logical row/column metadata plus the physical rectangle for one locker cell.

```rust
let cell = ParcelLockerCell::new(0, 1, CellRect::new(96, 20, 80, 60));
```

### CellStyle

Visual treatment for cell overlays. Use `overlay` for translucent status colors, `outline` for selected highlights, `opacity` for disabled adjustments, and `transparent` for a clear base style.

```rust
let available = CellStyle::overlay(Color::hex(0x00AA00), 80);
let selected = CellStyle::outline(Color::hex(0x00AEEF), 3);
let disabled = CellStyle::opacity(160);
```

### CellTap

Typed callback payload emitted by `ParcelLocker::on_cell_tap`.

```rust
locker.on_cell_tap(|tap| {
    let index = tap.index;
    let row = tap.row;
    let col = tap.col;
    let status = tap.status;
    let disabled = tap.disabled;
});
```
```

- [ ] **Step 4: Run documentation consistency checks**

Run:

```bash
grep -n "ParcelLocker" DSL_REFERENCE.md
grep -n "CellTap" DSL_REFERENCE.md
```

Expected: both commands print table-of-contents and section entries.

- [ ] **Step 5: Commit**

```bash
git add DSL_REFERENCE.md
git commit -m "docs: document parcel locker widget" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 8: Add playground support

**Files:**
- Modify: `DSL_PLAYGROUND.html`

- [ ] **Step 1: Add CSS for the preview**

In `DSL_PLAYGROUND.html`, near the ButtonMatrix styles, add:

```css
    /* ---- ParcelLocker ---- */
    .parcel-locker-shell {
      position: relative;
      width: min(100%, 360px);
      height: 240px;
      margin: 0 auto;
      border-radius: 18px;
      border: 1px solid var(--card-stroke);
      background:
        linear-gradient(90deg, rgba(255,255,255,.08) 1px, transparent 1px),
        linear-gradient(rgba(255,255,255,.08) 1px, transparent 1px),
        linear-gradient(135deg, #313244, #1e1e2e);
      background-size: 24px 24px, 24px 24px, auto;
      overflow: hidden;
    }
    .parcel-cell {
      position: absolute;
      display: grid;
      place-items: center;
      border: 1px solid rgba(255,255,255,.45);
      border-radius: 8px;
      font: 700 12px/1 var(--font-ui);
      color: #ffffff;
      transition: transform .15s ease, box-shadow .15s ease;
    }
    .parcel-cell.is-selected {
      box-shadow: 0 0 0 3px #00aeef, 0 0 18px rgba(0,174,239,.55);
      transform: translateY(-1px);
    }
    .parcel-cell.is-disabled {
      opacity: .45;
      filter: grayscale(.5);
    }
```

- [ ] **Step 2: Add quick link and section markup**

Add a quick link after ButtonMatrix:

```html
      <a href="#parcel-locker-playground">ParcelLocker</a>
```

Add this section after the ButtonMatrix playground section:

```html
      <section class="playground" id="parcel-locker-playground">
        <div class="playground-header">
          <div>
            <h2>3. ParcelLocker</h2>
            <p>Image-backed locker layout with clickable rectangular cells, caller-defined statuses, and single selection.</p>
          </div>
        </div>
        <div class="grid">
          <div class="preview-card">
            <div class="preview-title">Preview</div>
            <div class="preview-content" data-preview="parcelLocker"></div>
          </div>
          <div class="controls-card">
            <div class="preview-title">Controls</div>
            <label>
              <span class="label-row"><span>Selected cell</span><span class="readout" id="parcel-selected-readout">0</span></span>
              <input type="range" min="0" max="5" step="1" value="0" data-section="parcelLocker" data-key="selectedIndex" data-type="number">
            </label>
            <label>
              <span class="label-row"><span>Disabled cell</span><span class="readout" id="parcel-disabled-readout">2</span></span>
              <input type="range" min="0" max="5" step="1" value="2" data-section="parcelLocker" data-key="disabledIndex" data-type="number">
            </label>
            <label>
              <span class="label-row"><span>Available color</span></span>
              <input type="color" value="#00aa00" data-section="parcelLocker" data-key="availableColor" data-type="string">
            </label>
            <label>
              <span class="label-row"><span>Busy color</span></span>
              <input type="color" value="#ff8800" data-section="parcelLocker" data-key="busyColor" data-type="string">
            </label>
            <label>
              <span class="label-row"><span>Selected outline</span></span>
              <input type="color" value="#00aeef" data-section="parcelLocker" data-key="selectedColor" data-type="string">
            </label>
          </div>
        </div>
        <div class="code-card">
          <div class="preview-title">Generated Rust</div>
          <pre><code data-code="parcelLocker"></code></pre>
        </div>
      </section>
```

- [ ] **Step 3: Add playground state**

Inside the `const state = { ... }` object, after `buttonmatrix`, add:

```javascript
      parcelLocker: {
        selectedIndex: 0,
        disabledIndex: 2,
        availableColor: "#00aa00",
        busyColor: "#ff8800",
        selectedColor: "#00aeef",
      },
```

- [ ] **Step 4: Add renderer and code generator**

After `renderButtonMatrix()`, add:

```javascript
    function renderParcelLocker() {
      const s = state.parcelLocker;
      const cells = [
        { row: 0, col: 0, x: 16,  y: 18,  w: 88, h: 54, status: 1 },
        { row: 0, col: 1, x: 112, y: 18,  w: 88, h: 54, status: 2 },
        { row: 0, col: 2, x: 208, y: 18,  w: 88, h: 54, status: 1 },
        { row: 1, col: 0, x: 16,  y: 86,  w: 88, h: 118, status: 2 },
        { row: 1, col: 1, x: 112, y: 86,  w: 88, h: 118, status: 1 },
        { row: 1, col: 2, x: 208, y: 86,  w: 88, h: 118, status: 1 },
      ];
      const selected = Math.max(0, Math.min(cells.length - 1, s.selectedIndex));
      const disabled = Math.max(0, Math.min(cells.length - 1, s.disabledIndex));
      setReadout("parcel-selected-readout", String(selected));
      setReadout("parcel-disabled-readout", String(disabled));

      const preview = document.querySelector('[data-preview="parcelLocker"]');
      preview.innerHTML = `
        <div class="parcel-locker-shell">
          ${cells.map((cell, index) => {
            const color = cell.status === 1 ? s.availableColor : s.busyColor;
            const classes = [
              "parcel-cell",
              index === selected ? "is-selected" : "",
              index === disabled ? "is-disabled" : "",
            ].filter(Boolean).join(" ");
            return `<div class="${classes}" style="left:${cell.x}px;top:${cell.y}px;width:${cell.w}px;height:${cell.h}px;background:${hexToRgba(color, .55)};">${index}</div>`;
          }).join("")}
        </div>`;

      const cellLines = cells.map(cell =>
        `    ParcelLockerCell::new(${cell.row}, ${cell.col}, CellRect::new(${cell.x}, ${cell.y}, ${cell.w}, ${cell.h})),`
      );
      const statusLines = cells.map((cell, index) =>
        `locker.set_status(${index}, CellStatusId(${cell.status}));`
      );
      const code = [
        "static LOCKER_CELLS: &[ParcelLockerCell] = &[",
        ...cellLines,
        "];",
        "",
        "let bg = ImageSrc::file(c\"/lfs/locker.bin\");",
        "",
        "let locker = ParcelLocker::new(&screen, 2, 3, LOCKER_CELLS)",
        "    .background(&bg)",
        "    .default_style(CellStyle::overlay(Color::hex(0x000000), 40))",
        `    .status_style(CellStatusId(1), CellStyle::overlay(Color::hex(0x${s.availableColor.slice(1).toUpperCase()}), 80))`,
        `    .status_style(CellStatusId(2), CellStyle::overlay(Color::hex(0x${s.busyColor.slice(1).toUpperCase()}), 80))`,
        `    .selected_style(CellStyle::outline(Color::hex(0x${s.selectedColor.slice(1).toUpperCase()}), 3))`,
        "    .size(Size::Px(320), Size::Px(240));",
        "",
        ...statusLines,
        `locker.set_disabled(${disabled}, true);`,
        `locker.set_selected(Some(${selected}));`,
        "locker.on_cell_tap(|tap| {",
        "    let _index = tap.index;",
        "    let _row = tap.row;",
        "    let _col = tap.col;",
        "    let _status = tap.status;",
        "    let _disabled = tap.disabled;",
        "});",
      ].join("\n");
      setCode("parcelLocker", code);
    }
```

Add this helper near the other JavaScript helpers:

```javascript
    function hexToRgba(hex, alpha) {
      const clean = hex.replace("#", "");
      const value = Number.parseInt(clean, 16);
      const r = (value >> 16) & 255;
      const g = (value >> 8) & 255;
      const b = value & 255;
      return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    }
```

Update `renderAll()` to call:

```javascript
      renderParcelLocker();
```

- [ ] **Step 5: Verify playground text references**

Run:

```bash
grep -n "parcelLocker" DSL_PLAYGROUND.html
grep -n "ParcelLocker" DSL_PLAYGROUND.html
```

Expected: both commands print entries for state, renderer, generated code, markup, and quick link.

- [ ] **Step 6: Commit**

```bash
git add DSL_PLAYGROUND.html
git commit -m "docs: add parcel locker playground" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 9: Final verification

**Files:**
- Verify: entire repository

- [ ] **Step 1: Run the full test suite**

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

Expected: PASS. If it fails, run `cargo fmt`, inspect `git --no-pager diff`, then repeat `cargo fmt --check`.

- [ ] **Step 3: Inspect final diff**

Run:

```bash
git --no-pager diff --stat HEAD
git --no-pager status --short
```

Expected: the working tree is clean after the previous task commits.
