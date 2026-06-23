# ParcelLocker Widget — Design

Status: approved for planning
Crate: `lvgl-dsl`
Target: `no_std` + `extern crate alloc` (Zephyr embedded + desktop-sim)
LVGL version: 9.2.x

## 1. Problem & Goals

Add an interactive parcel-locker layout widget. The caller provides a locker
background image plus an `N x M` logical matrix of cells. Each cell has its own
rectangle over the image, because real locker layouts can have uneven doors.

When a user taps the layout, the widget identifies the cell under the tap and
emits a typed callback payload. The application receives the cell index plus
row/column metadata and decides what business action to take.

### In scope

- A new `ParcelLocker` composite widget exported from `lvgl::mod` and
  `lvgl::prelude`.
- Background image support using the existing `ImageSrc` / `Widget::bg_image`
  pattern.
- Per-cell rectangular hit areas relative to the widget's container.
- Input-order cell indexes plus row/column metadata in callback payloads.
- Caller-defined status IDs mapped to configurable visual styles.
- Single-selection highlighting by default.
- APIs to set status, selected cell, disabled state, and style mappings.
- Disabled cells remain tappable and report `disabled: true` in callbacks.
- Desktop/mock bindings, spy calls, and unit tests matching existing widget
  patterns.
- `DSL_REFERENCE.md` and `DSL_PLAYGROUND.html` updates.

### Out of scope

- Automatic image analysis or locker-cell detection.
- Arbitrary polygon hit regions.
- Multi-select behavior in the first implementation.
- Scroll/zoom/pan support for oversized locker images.
- A fixed business enum such as `Available`, `Occupied`, or `Reserved`.
- Replacing `ButtonMatrix`; this widget solves image-backed irregular layouts.

## 2. Public API

The public API will stay close to existing LVGL DSL wrappers while exposing a
typed domain model for cells.

```rust
use lvgl_dsl::lvgl::prelude::*;

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

locker
    .set_status(0, CellStatusId(1))
    .set_disabled(1, true)
    .set_selected(Some(0))
    .on_cell_tap(|tap| {
        let _index = tap.index;
        let _row = tap.row;
        let _col = tap.col;
        let _status = tap.status;
        let _disabled = tap.disabled;
    });
```

Proposed core types:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellStatusId(pub u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ParcelLockerCell {
    pub row: usize,
    pub col: usize,
    pub rect: CellRect,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellTap {
    pub index: usize,
    pub row: usize,
    pub col: usize,
    pub status: CellStatusId,
    pub disabled: bool,
}
```

`CellStyle` will describe the visual treatment that can be applied to a cell
overlay: background color/opacity, border/outline color, width, and opacity. It
will use existing `Color` values and LVGL style calls rather than adding a new
color abstraction.

## 3. Architecture

Use a composite overlay implementation:

1. `ParcelLocker` owns a root `Obj` container.
2. The root container receives the locker image via `bg_image`.
3. The widget creates one child `Obj` overlay for every `ParcelLockerCell`.
4. Each overlay is positioned and sized from its `CellRect`.
5. Each overlay is clickable and has an event trampoline registered on it.
6. The trampoline recovers the owning `ParcelLocker` context and cell index,
   builds a `CellTap`, and dispatches the caller callback.

This approach fits arbitrary locker geometries and avoids relying on pointer
coordinate APIs that the current DSL and mock bindings do not expose. It also
keeps styling simple: each cell maps to a real LVGL object that can receive
background, border, outline, opacity, disabled, and clickable state updates.

The callback implementation will follow the pinned context pattern used by
`SearchBar`, not the simple `Widget::on_click(fn(Event))` API. `ParcelLocker`
needs object-owned state and typed callback storage, so a boxed context with
LVGL `user_data` is the right boundary.

## 4. State and Styling

`ParcelLocker` keeps runtime state for each cell:

- static geometry and row/column metadata;
- current `CellStatusId`;
- disabled flag;
- selected flag;
- overlay LVGL object pointer or wrapper.

Changing status, selection, or disabled state restyles only the affected
overlay. Single selection is built in: `set_selected(Some(index))` clears the
previous selected cell before highlighting the new one, and
`set_selected(None)` clears selection.

Style precedence is deterministic:

1. default style;
2. status style;
3. disabled style adjustments;
4. selected style adjustments.

Disabled cells stay clickable because the application wants to receive the tap
and decide what to do. The widget will visually distinguish disabled cells and
include `disabled: true` in `CellTap`.

## 5. Validation and Errors

Construction validates the layout before creating overlays:

- cell list is not empty;
- matrix dimensions are non-zero;
- each rectangle has positive width and height;
- each row is less than `rows`;
- each column is less than `cols`;
- each `(row, col)` appears at most once.

Invalid construction data will panic with clear messages, matching existing
DSL wrapper misuse checks such as `ButtonMatrix::ctrl_map()` ordering and button
width validation. Index-based methods such as `set_status`, `set_disabled`, and
`set_selected(Some(index))` will also panic on out-of-range indexes.

Missing status-style mappings will not fail. They fall back to the default
style, so callers can add business statuses incrementally without leaving cells
unstyled.

## 6. Bindings and Tests

The real Zephyr path will rely on bindgen for LVGL symbols already available
through LVGL. The desktop/mock path will need any missing object-positioning,
style, event, and user-data calls required by the composite widget.

Unit tests will follow the existing wrapper style:

- reset the object pool and create a `Screen`;
- construct a `ParcelLocker`;
- drain spy calls;
- call one public method;
- assert the expected LVGL spy call or state transition.

Core test cases:

- construction creates one root object plus one overlay per cell;
- background image is applied to the root;
- overlay geometry is set from each `CellRect`;
- `set_status` restyles the target overlay;
- `set_selected(Some(index))` highlights one cell and clears the prior one;
- `set_selected(None)` clears selection;
- `set_disabled(index, true)` updates disabled visual state but does not suppress
  callbacks;
- callback payload includes index, row, column, status, and disabled flag;
- invalid dimensions, rectangles, duplicate coordinates, and indexes fail
  clearly.

## 7. Documentation and Playground

`DSL_REFERENCE.md` will add `ParcelLocker` to the widget list, document the
cell geometry model, describe status-style mapping, and include a complete code
example.

`DSL_PLAYGROUND.html` will add a visual section for configuring a sample
background preview, cell rectangles, statuses, disabled cells, and selected
cell. The generated code will use explicit `ParcelLockerCell` definitions so
users see that the layout is image-relative and not a regular equal-size grid.
