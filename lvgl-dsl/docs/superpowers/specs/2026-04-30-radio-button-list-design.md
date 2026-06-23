# RadioButtonList Widget - Design

Status: approved for planning
Crate: `lvgl-dsl`
Target: `no_std` + `extern crate alloc` (Zephyr embedded + desktop-sim)
LVGL version: 9.2.x

## 1. Problem & Goals

Add a reusable radio button list widget for screens that need one selected
choice from a vertical set of options. The visual target is a list of large
clickable rows/cards with a circular indicator at the leading edge and text
beside it. The widget must provide the behavior and structure, while callers
remain responsible for applying product-specific colors, spacing, and typography.

### In scope

- A new `RadioButtonList` composite widget exported from `lvgl::mod` and
  `lvgl::prelude`.
- Runtime `&str` option labels. LVGL copies label text, so callers do not need
  static C string maps.
- Fixed row height configured by the caller for predictable embedded layouts.
- Configurable row gap, row padding, indicator size, indicator-label gap, and
  label alignment.
- Low-level style structs/setters for list, row, selected row, indicator,
  selected indicator, and label styling.
- Automatic single selection on row tap.
- `selected() -> Option<usize>` and `set_selected(Option<usize>)` APIs.
- Optional typed callback invoked after auto-selection.
- Per-option enabled/disabled state. Disabled options do not auto-select and do
  not call the callback.
- Clear panics for invalid construction data or out-of-range indexes.
- Desktop/mock bindings, spy calls, and unit tests matching existing composite
  widget patterns.
- Reference documentation and playground updates for the new widget.

### Out of scope

- Product-specific Jetbeep theme defaults.
- Multi-select behavior.
- Radio groups spanning multiple independent lists.
- Virtualized lists for hundreds of options.
- Wrapping LVGL `lv_checkbox` as the public API.
- ButtonMatrix-style virtual buttons.

## 2. Public API

The API should feel like the existing DSL wrappers while exposing typed helpers
for the radio-list-specific pieces.

```rust
use lvgl_dsl::lvgl::prelude::*;

let list = RadioButtonList::new(&screen, &[
    "Choose another locker",
    "Cancel placement",
    "Locker didn't open",
    "Locker is occupied",
])
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

Proposed core types:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RadioButtonEvent<'a> {
    pub index: usize,
    pub label: &'a str,
}

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

pub struct RadioIndicatorStyle {
    pub bg_color: Option<Color>,
    pub bg_opa: Option<u8>,
    pub border_color: Option<Color>,
    pub border_width: Option<i32>,
    pub border_opa: Option<u8>,
    pub radius: Option<CornerRadius>,
}
```

The public API will expose these setters and accessors:

- `row_style(style)`
- `selected_row_style(style)`
- `indicator_style(style)`
- `selected_indicator_style(style)`
- `label_style(style)`
- `set_selected(selection)`
- `selected()`
- `set_enabled(index, enabled)`
- `is_enabled(index)`
- `on_changed(callback)`

`RadioButtonList` itself implements `Widget` by returning its root LVGL object,
so callers can still use ordinary layout and positioning methods such as
`size`, `align`, `flex_grow`, and `set_hidden`.

## 3. Architecture

Use a composite card-row implementation:

1. `RadioButtonList` owns a root `Obj` container.
2. The root uses `LV_FLEX_FLOW_COLUMN` and caller-configured row gap.
3. For each option, the widget creates:
   - a clickable row `Obj`;
   - a child indicator `Obj`;
   - a child label `Label`.
4. Each row uses fixed caller-configured height and full-width sizing.
5. Each row stores enough event context to identify its option index.
6. On click, the event trampoline checks whether the row is enabled, updates
   single selection, refreshes affected row/indicator styles, and invokes the
   callback if present.

This approach matches the screenshots better than a direct `lv_checkbox`
wrapper because the whole row can be styled as a card and the circular indicator
can be represented as an ordinary child object. It also avoids expanding the DSL
around LVGL part selectors before there is a broader design for part-specific
styling.

The callback implementation should follow the object-owned context pattern used
by `SearchBar` rather than the simple `Widget::on_click(fn(Event))` helper. The
widget needs owned labels, enabled flags, selected state, child object pointers,
and a typed callback.

## 4. State and Behavior

Selection is single-choice:

- `selected()` returns the current selected index, or `None`.
- `set_selected(Some(index))` selects the given option and clears the previous
  selected option.
- `set_selected(None)` clears selection.
- Clicking an enabled row auto-selects that row before dispatching the typed
  callback.
- Clicking the already-selected enabled row keeps it selected and still
  dispatches the callback, so applications can react consistently to user taps.

Enabled state is per option:

- Options are enabled by default.
- `set_enabled(index, false)` marks the option disabled.
- Disabled rows remain visible but do not auto-select and do not dispatch the
  typed callback.
- If the currently selected option is disabled, selection is preserved until the
  caller changes it. This avoids surprising state loss from a style/availability
  update.

Construction and index methods panic on invalid misuse:

- empty option list;
- non-positive row height or indicator size;
- out-of-range selected index;
- out-of-range enabled-state index.

This matches existing wrapper behavior where invalid DSL construction panics
with a clear message.

## 5. Styling

The widget provides behavior-first defaults and leaves product visuals to
callers. Defaults should be simple and legible:

- root: transparent column container;
- row: full-width clickable object with no product-specific color;
- indicator: hollow circle using existing neutral style values;
- selected indicator: visually distinct from unselected using only neutral
  defaults unless the caller supplies colors;
- label: ordinary LVGL label text.

Style application has deterministic precedence:

1. base row/indicator/label styles;
2. enabled or disabled visual adjustment;
3. selected row/indicator styles.

Style structs use existing primitives such as `Color` and `CornerRadius`; they
do not introduce a new color abstraction. During implementation, if a needed
style property is not currently covered by `Widget`, add the narrow missing
binding and mock spy coverage rather than widening the widget with unrelated
styling APIs.

## 6. Bindings and Tests

The real Zephyr path continues to use bindgen for LVGL symbols. The desktop/mock
path may need additional spy coverage for object positioning, user data, and any
style setters not already recorded.

Unit tests should follow the existing wrapper style:

- reset the object pool and create a `Screen`;
- construct a `RadioButtonList`;
- drain spy calls;
- call one public method;
- assert the expected LVGL spy call or public state transition.

Core test cases:

- construction creates one root object plus row, indicator, and label children
  for each option;
- runtime labels are copied into child labels;
- root is configured as a column list and rows use the configured fixed height;
- `set_selected(Some(index))` updates state and restyles old/new rows;
- `set_selected(None)` clears selection and restyles the previous row;
- clicking an enabled row auto-selects before invoking the callback;
- clicking a disabled row does not select or call the callback;
- disabling the currently selected row preserves selected state;
- invalid empty options, invalid sizes, and invalid indexes panic clearly;
- exports are available from both `lvgl` and `lvgl::prelude`.

## 7. Documentation and Playground

`DSL_REFERENCE.md` will add `RadioButtonList` to the widget list, document the
runtime-label API, show how selection and disabled state work, and include a
short styling example.

`DSL_PLAYGROUND.html` will add a visual section for a radio button list with
editable option labels, selected index, disabled rows, row height, gap, and
basic style values. The generated code should use runtime `&str` labels and the
same selection APIs as the real widget.
