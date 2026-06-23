# LVGL ButtonMatrix — Design

Status: approved for planning
Crate: `lvgl-dsl`
Target: `no_std` + `extern crate alloc` (Zephyr embedded + desktop-sim)
LVGL version: **9.2.x**

## 1. Problem & Goals

Add a safe, ergonomic DSL wrapper for LVGL's `lv_buttonmatrix` widget and keep the public reference documentation and HTML playground in sync with the new API.

The button matrix displays many virtual buttons from a NULL-terminated text map. LVGL stores pointers to that map rather than owning Rust allocations, so the wrapper should prioritize static, lifetime-safe maps built from Rust `c"..."` literals.

### In scope

- `ButtonMatrix` widget wrapper implementing the existing `Widget` trait.
- Static text map support with `ButtonMatrixMapEntry::new(c"...")` and `type ButtonMatrixMap = [ButtonMatrixMapEntry]`.
- Static control map support with `type ButtonMatrixCtrlMap = [u32]`.
- Core LVGL API coverage:
  - `map(&'static ButtonMatrixMap)`
  - `ctrl_map(&'static ButtonMatrixCtrlMap)`
  - `button_width(button_id, width)`
  - `set_button_ctrl(button_id, ctrl)` / `clear_button_ctrl(button_id, ctrl)`
  - `set_button_ctrl_all(ctrl)` / `clear_button_ctrl_all(ctrl)`
  - `one_checked(bool)`
  - `get_selected_button() -> Option<u32>`
  - `get_button_text(button_id) -> Option<&CStr>`
- Public constants for common `LV_BUTTONMATRIX_CTRL_*` flags and width values.
- Exports through `lvgl::mod` and `lvgl::prelude`.
- Desktop/mock bindings, spy calls, and unit tests matching existing widget patterns.
- `DSL_REFERENCE.md` and `DSL_PLAYGROUND.html` updates.

### Out of scope

- Owned runtime maps from `&[&str]`.
- Draw-task customization APIs.
- Enforcing text-map/ctrl-map length matching at runtime.
- Replacing or refactoring the existing keyboard-specific map types.

## 2. Public API

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

let matrix = ButtonMatrix::new(&screen)
    .map(NUMPAD_MAP)
    .ctrl_map(NUMPAD_CTRL)
    .one_checked(true)
    .button_width(6, 2)
    .on_event(|event| {
        let _ = event;
    }, LvEventCode::ValueChanged);
```

`ButtonMatrixMapEntry` mirrors `KeyMapEntry`: a transparent wrapper around a `*const c_char`, `Copy + Clone`, with `unsafe impl Send` and `Sync` justified by construction from `'static` C string literals.

`ButtonMatrixCtrlMap = [u32]` matches the existing keyboard `CtrlMap` and the crate's desktop binding signature. Width and flag constants stay as raw `u32` values so callers can combine them with bitwise OR.

## 3. Safety and Invariants

Maps and control maps must be `'static` because LVGL keeps their pointers after `lv_buttonmatrix_set_map` and `lv_buttonmatrix_set_ctrl_map` return. The safe constructor for entries only accepts `&'static CStr`, so ordinary use with `c"..."` literals satisfies this requirement.

Button map arrays must end with `c""` and use `c"\n"` for row breaks. Control maps must include one entry per actual button, excluding row breaks and the terminator. A mismatch is documented as caller error because LVGL's C API does not expose a cheap validation hook, and counting every map on embedded targets would add runtime work to a static-data API.

`get_selected_button()` maps the LVGL sentinel `LV_BUTTONMATRIX_BUTTON_NONE` (`0xFFFF`) to `None`. `get_button_text()` returns `None` if LVGL returns a null pointer.

## 4. Bindings and Tests

The Zephyr path continues to use bindgen output. The desktop-sim/mock path needs declarations and spy coverage for:

- `lv_buttonmatrix_create`
- `lv_buttonmatrix_set_map`
- `lv_buttonmatrix_set_ctrl_map`
- `lv_buttonmatrix_set_button_width`
- `lv_buttonmatrix_set_button_ctrl`
- `lv_buttonmatrix_clear_button_ctrl`
- `lv_buttonmatrix_set_button_ctrl_all`
- `lv_buttonmatrix_clear_button_ctrl_all`
- `lv_buttonmatrix_set_one_checked`
- `lv_buttonmatrix_get_selected_button`
- `lv_buttonmatrix_get_button_text`

Unit tests will follow the existing wrapper style: reset the object pool, create a `Screen`, create `ButtonMatrix`, drain spy calls, invoke one method, then assert that the expected `LvCall` was recorded. Getter tests will verify the default no-selection state and selected/text mock behavior.

## 5. Documentation and Playground

`DSL_REFERENCE.md` will add ButtonMatrix to the widget list, include method tables and examples, and document the static map lifetime requirements clearly. It will also add supporting-type entries for `ButtonMatrixMapEntry`, `ButtonMatrixMap`, and `ButtonMatrixCtrlMap` plus the common flag/width constants.

`DSL_PLAYGROUND.html` will add a ButtonMatrix quick link and section. The preview will approximate LVGL's rows and virtual buttons with configurable labels, selection, one-checked mode, common control flags, and action-button widths. The generated code will use static `c"..."` arrays so it matches the real API rather than implying runtime-owned strings are supported.
