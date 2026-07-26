# lvgl-dsl — DSL Reference

A safe, ergonomic Rust DSL that wraps the [LVGL](https://lvgl.io/) v9 embedded graphics library. All widget configuration uses method chaining on a shared `Widget` trait, so the same style, layout, and event API is available on every element.

## Table of Contents

- [lvgl-dsl — DSL Reference](#lvgl-dsl--dsl-reference)
  - [Table of Contents](#table-of-contents)
  - [Getting Started](#getting-started)
    - [Minimal Example](#minimal-example)
  - [Widgets](#widgets)
    - [Screen](#screen)
    - [Obj](#obj)
    - [Button](#button)
    - [ButtonMatrix](#buttonmatrix)
    - [ParcelLocker](#parcellocker)
    - [Label](#label)
    - [Dropdown](#dropdown)
    - [QrCode](#qrcode)
    - [Image](#image)
    - [ImageButton](#imagebutton)
    - [TextArea](#textarea)
    - [PhoneFormatterField](#phoneformatterfield)
    - [Keyboard](#keyboard)
    - [SearchBar](#searchbar)
    - [Arc](#arc)
    - [RadioButtonList](#radiobuttonlist)
  - [Animation](#animation)
    - [Anim Builder](#anim-builder)
    - [AnimHandle](#animhandle)
    - [Path](#path)
  - [Widget Trait — Shared API](#widget-trait--shared-api)
    - [Alignment \& Position](#alignment--position)
    - [Flexbox Layout](#flexbox-layout)
    - [Sizing](#sizing)
    - [Background](#background)
    - [Background Image](#background-image)
    - [Text](#text)
    - [Shape \& Opacity](#shape--opacity)
    - [Border](#border)
    - [Outline](#outline)
    - [Shadow](#shadow)
    - [Events](#events)
    - [State](#state)
    - [Flags](#flags)
    - [Convenience Setters/Getters](#convenience-settersgetters)
    - [Raw Pointer Access](#raw-pointer-access)
    - [Cleanup](#cleanup)
  - [Supporting Types](#supporting-types)
    - [Color](#color)
    - [Palette](#palette)
    - [Size](#size)
    - [LvAlign](#lvalign)
    - [FlexFlow](#flexflow)
    - [FlexAlign](#flexalign)
    - [CornerRadius](#cornerradius)
    - [BorderSide](#borderside)
    - [LvDropdownDir](#lvdropdowndir)
    - [Font](#font)
    - [ImageSrc](#imagesrc)
    - [ImageButtonState](#imagebuttonstate)
    - [ButtonMatrixMapEntry / ButtonMatrixMap](#buttonmatrixmapentry--buttonmatrixmap)
    - [ButtonMatrixCtrlMap](#buttonmatrixctrlmap)
    - [CellStatusId](#cellstatusid)
    - [CellRect](#cellrect)
    - [ParcelLockerCell](#parcellockercell)
    - [CellStyle](#cellstyle)
    - [CellTap](#celltap)
    - [LvState](#lvstate)
    - [LvObjFlag](#lvobjflag)
    - [Event](#event)
    - [LvEventCode](#lveventcode)
    - [ScreenAnim](#screenanim)
    - [KeyboardLayout](#keyboardlayout)
    - [LvKeyboardMode](#lvkeyboardmode)
    - [KeyboardLocale](#keyboardlocale)
    - [KeyMap / KeyMapEntry](#keymap--keymapentry)
    - [CtrlMap](#ctrlmap)
    - [KeyboardTheme](#keyboardtheme)
    - [LocaleSwitcher](#localeswitcher)

---

## Getting Started

Import everything with the prelude:

```rust
use lvgl_dsl::prelude::*;
```

This brings in all widgets, types, and the `Widget` trait.

### Minimal Example

```rust
use lvgl_dsl::prelude::*;

let screen = Screen::new();

let container = Obj::new(&screen)
    .flex_col()
    .flex_align(FlexAlign::Center, FlexAlign::Center, FlexAlign::Start)
    .gap(12)
    .size(Size::Pct(100), Size::Pct(100));

let btn = Button::new(&container)
    .width(Size::Pct(80))
    .height(Size::Px(50))
    .bg_color(Color::palette(Palette::Blue))
    .radius(CornerRadius::Px(8))
    .on_click(|_| { /* handle click */ });

btn.text("Press me");

screen.load();
```

---

## Widgets

### Screen

A top-level LVGL display container. All other widgets are parented to a screen, directly or indirectly. There is always one active screen.

**Construction**

```rust
// Get the currently active screen
let screen = Screen::active();

// Create a new blank screen
let screen = Screen::new();
```

**Methods**

| Method | Description |
|--------|-------------|
| `load()` | Immediately replaces the active screen with this one. |
| `load_anim(anim, duration_ms, delay_ms, auto_delete)` | Transitions to this screen with an animation. See [`ScreenAnim`](#screenanim). |
| `unsafe load_ptr(ptr, anim, duration_ms, delay_ms, auto_delete)` | Loads a screen from a raw `lv_obj_t *` stored as `usize` (obtained via `raw_ptr()`). Unsafe — the pointer must be valid and non-null. |

> **Warning — `auto_delete`:** When `auto_delete = true`, LVGL deletes the previous screen's C objects after the transition. Any Rust widget handles pointing to the old screen become dangling pointers. Do not access them after calling `load_anim`.

**Example**

```rust
let next = Screen::new();
// ... populate next ...
next.load_anim(ScreenAnim::FadeIn, 300, 0, true);
```

---

### Obj

A plain, transparent LVGL container object (`lv_obj_create`). Has no built-in visual content — useful as a layout container or styled panel.

**Construction**

```rust
let obj = Obj::new(&parent);
```

**Example**

```rust
let panel = Obj::new(&screen)
    .size(Size::Pct(100), Size::Px(80))
    .bg_color(Color::hex(0x1E1E2E))
    .radius(CornerRadius::Px(12))
    .flex_row()
    .pad_all(8)
    .gap(8);
```

---

### Button

An interactive LVGL button widget (`lv_button_create`). Inherits all `Widget` trait methods and adds a convenience method for adding a centred text label.

**Construction**

```rust
let btn = Button::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `text(str)` | Creates an internal `lv_label` child centred on the button. The label is owned by LVGL's widget tree. For independent label control use `Label::new(&btn)` instead. |
| `icon(&ImageSrc)` | Creates an internal `Image` child centred on the button. The image is owned by LVGL's widget tree. For independent image control use `Image::new(&btn)` instead. |
| `loading_config(ButtonLoadingConfig)` | Configures the loading-state presentation and minimum visible duration for this button. |
| `set_loading(bool)` | Starts or finishes loading using the configured minimum duration. |
| `start_loading() -> LoadingHandle` | Starts loading and returns a handle whose `finish()` method completes loading with minimum-duration handling. |
| `is_loading() -> bool` | Returns whether the button is currently showing loading content. |

**Example**

```rust
let btn = Button::new(&container)
    .width(Size::Px(200))
    .height(Size::Px(48))
    .bg_color(Color::palette(Palette::Green))
    .radius(CornerRadius::Full)
    .on_click(|_| { /* ... */ });

btn.text("Confirm");

// Button with an image icon (safe — file path):
let src = ImageSrc::file_cstr(c"/lfs/icons/check.bin");
btn.icon(&src);

// Button with an image icon (unsafe — C descriptor):
// extern "C" { static MY_ICON: core::ffi::c_void; }
// let src = unsafe { ImageSrc::descriptor(&raw const MY_ICON) };
// btn.icon(&src);
```

**Loading state**

```rust
let btn = Button::new(&container)
    .width(Size::Px(280))
    .height(Size::Px(64))
    .radius(CornerRadius::Full)
    .loading_config(
        ButtonLoadingConfig::new()
            .text("LOADING...")
            .min_duration_ms(300)
            .indicator(ButtonLoadingIndicator::Spinner {
                size_px: 36,
                spin_ms: 900,
                arc_length_deg: 90,
            })
            .gap_px(12),
    );

btn.text("Confirm");
btn.set_loading(true);
// Later, when the operation completes:
btn.set_loading(false);
```

Use a managed handle when the loading operation has a clear owner:

```rust
let loading = btn.start_loading();
// Perform work...
loading.finish();
```

Lifecycle notes:

- `finish()` and dropping an unfinished `LoadingHandle` both respect `min_duration_ms`; if the minimum has not elapsed yet, restoration is deferred until the timer fires.
- If LVGL deletes the button or its parent screen while loading is active, the loading state cancels its timer and makes outstanding handles stale no-ops.
- If the loading container is deleted externally, the next finish restores the button state without deleting that container again.
- If a pre-existing direct child is deleted while loading is active, restore skips that child instead of touching its stale LVGL pointer.
- `custom_content(fn(&LvObj))` accepts a plain function pointer, so custom builders cannot capture local variables directly.

Use a static or rotating image as the indicator:

```rust
let src = ImageSrc::file_cstr(c"/lfs/icons/sync.bin");
btn.loading_config(
    ButtonLoadingConfig::new()
        .text("SYNCING")
        .indicator(ButtonLoadingIndicator::Image {
            src,
            size_px: 32,
            rotation_ms: 800,
        }),
);
```

Use a custom child builder for project-specific loading content. This is also
the extension point for future widgets such as Lottie once a dedicated wrapper
exists:

```rust
fn loading_content(parent: &LvObj) {
    Label::new(parent).text("Please wait");
}

btn.loading_config(
    ButtonLoadingConfig::new()
        .indicator(ButtonLoadingIndicator::None)
        .custom_content(loading_content),
);
```

**Styling the loading container, label, and spinner**

Use the `container_style`, `label_style`, and `spinner_style` hooks to
fully control the loading appearance without writing a `custom_content`
builder. Each hook receives the freshly-created widget so it can apply
arbitrary styles. The container is sized to fully cover the button (with
its own padding/border zeroed and `SCROLLABLE` removed), so container
styling completely replaces the button's surface during loading.

```rust
fn loading_container(c: &LvObj) {
    c.bg_color(Palette::GREY_200).radius(CornerRadius::Full);
    c.border_width(0).outline_width(0);
}

fn loading_label(l: &Label) {
    l.text_color(Color::hex(0x4A4A4A));
}

fn loading_spinner(s: &Spinner) {
    s.track_color(Color::hex(0xE0E0E0))
        .indicator_color(Color::hex(0x4A4A4A))
        .track_width(4)
        .indicator_width(4);
}

btn.loading_config(
    ButtonLoadingConfig::new()
        .text("LOADING...")
        .container_style(loading_container)
        .label_style(loading_label)
        .spinner_style(loading_spinner),
);
```

The hooks are plain function pointers (no captured state). `spinner_style`
is invoked only when the indicator is `ButtonLoadingIndicator::Spinner`.

---

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
| `map(&'static ButtonMatrixMap)` | Sets the static text map. The map must end with `c""`; use `c"\n"` for row breaks. LVGL keeps a pointer to this array. Panics if the terminator is missing. |
| `ctrl_map(&'static ButtonMatrixCtrlMap)` | Sets width/control values for each actual button, excluding row breaks and the terminator. Panics if called before `map()` or if the control map length does not match the actual button count. |
| `button_width(button_id, width)` | Sets one button's relative width (`1..=15`) inside its row. Prefer `ctrl_map` for initial layout. Panics if `width` is outside `1..=15`. |
| `set_button_ctrl(button_id, ctrl)` | Sets one or more `BUTTONMATRIX_CTRL_*` flags on a button. |
| `clear_button_ctrl(button_id, ctrl)` | Clears one or more flags from a button. |
| `set_button_ctrl_all(ctrl)` | Sets one or more flags on every button. |
| `clear_button_ctrl_all(ctrl)` | Clears one or more flags from every button. |
| `one_checked(bool)` | Enables radio-button-like behavior where only one checkable button can be checked at a time. |
| `get_selected_button() -> Option<u32>` | Returns the most recently activated button, or `None` if LVGL has no selection. |
| `get_button_text(button_id) -> Option<&CStr>` | Returns a button's text, or `None` if LVGL returns a null pointer. |

**Example**

```rust
use lvgl_dsl::prelude::*;

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

> **Lifetime rule:** `ButtonMatrixMap` and `ButtonMatrixCtrlMap` must be static because LVGL keeps pointers to both after `map()` / `ctrl_map()` return.

---

### ParcelLocker

An image-backed interactive parcel-locker layout. `ParcelLocker` creates a root LVGL object that can be given a background image and one clickable overlay object per locker cell. Each cell has its own rectangle, so layouts can represent uneven physical locker doors rather than only equal-size grids.

**Construction**

```rust
static LOCKER_CELLS: &[ParcelLockerCell] = &[
    ParcelLockerCell::new(0, 0, CellRect::new(10, 20, 80, 60)),
    ParcelLockerCell::new(0, 1, CellRect::new(96, 20, 80, 60)),
    ParcelLockerCell::new(1, 0, CellRect::new(10, 86, 80, 120)),
];

let bg = ImageSrc::file_cstr(c"/lfs/locker.bin");

let locker = ParcelLocker::new(&screen, 2, 2, LOCKER_CELLS);

locker
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

**Index semantics**

All `index` parameters and `CellTap::index` refer to the zero-based position in the `cells` slice passed to `ParcelLocker::new`. It is not necessarily `row * cols + col`. The `row` and `col` fields are validated metadata returned in `CellTap`.

**Tap handling**

Disabled cells are still clickable. The callback receives `disabled: true`, allowing application logic to show a message, reject the action, or handle a special workflow.

---

### Label

A text display widget (`lv_label_create`).

**Construction**

```rust
let label = Label::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `text(str)` | Sets the label text. LVGL copies the string, so the `str` value does not need to outlive this call. |

**Example**

```rust
let lbl = Label::new(&screen)
    .text_color(Color::white())
    .text_font(&Font::montserrat_30())
    .align(LvAlign::Center, 0, -40);

lbl.text("Hello, World!");
```

---

### Dropdown

A selectable list widget (`lv_dropdown_create`). Displays a button that opens a scrollable option list when pressed. The list direction, height cap, and expand symbol are all configurable.

Requires `CONFIG_LV_USE_DROPDOWN=y` in the LVGL/Kconfig configuration.

**Construction**

```rust
let dd = Dropdown::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `options(str)` | Sets the option list from a `"\n"`-delimited string, e.g. `"English\nFrench\nGerman"`. LVGL copies the string. |
| `selected(index: u16)` | Selects the option at `index` (zero-based). LVGL clamps out-of-range values to the last valid index. |
| `open()` | Programmatically opens the option list. |
| `close()` | Programmatically closes the option list. |
| `direction(LvDropdownDir)` | Controls which direction the list opens. Defaults to [`LvDropdownDir::Down`](#lvdropdowndir). |
| `max_height(px: i32)` | Constrains the maximum pixel height of the open list. Pass `0` to remove the limit. |
| `symbol(str)` | Sets the expand-arrow symbol shown on the button. Pass `""` to suppress the symbol. LVGL copies the string. |
| `get_selected() -> u16` | Returns the zero-based index of the currently selected option. |
| `unsafe selected_from_raw_ptr(usize) -> u16` | Returns the selected index from a raw `lv_obj_t *` stored as `usize`. Unsafe — the pointer must be valid. |

**Example**

```rust
let dd = Dropdown::new(&container)
    .options("English\nFrench\nGerman\nSpanish")
    .selected(1)
    .direction(LvDropdownDir::Down)
    .max_height(200)
    .symbol("▼")
    .width(Size::Px(200))
    .on_event(|_| { /* selection changed */ }, LvEventCode::ValueChanged);
```

---

### QrCode

A canvas-based QR code widget (`lv_qrcode_create`). Requires `CONFIG_LV_USE_QRCODE=y` and `CONFIG_LV_USE_CANVAS=y` in the LVGL/Kconfig configuration.

**Construction**

```rust
let qr = QrCode::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `set_size(px: i32)` | Sets the square canvas size in pixels. Must be called before `update()`. |
| `dark_color(Color)` | Sets the foreground (module) colour. Default is black. |
| `light_color(Color)` | Sets the background colour. Default is white. |
| `update(data: &[u8]) -> Result<(), ()>` | Encodes and renders `data`. Returns `Err(())` if the data is too long for the selected size. |

**Example**

```rust
let qr = QrCode::new(&screen)
    .set_size(200)
    .dark_color(Color::black())
    .light_color(Color::white())
    .align(LvAlign::Center, 0, 0);

qr.update(b"https://example.com").expect("data too long");
```

---

### Image

A static image widget (`lv_image_create`). Displays an image from a C array descriptor or a file-system path. Supports offset, uniform scaling, and rotation.

Requires `CONFIG_LV_USE_IMAGE=y` in the LVGL/Kconfig configuration.

**Construction**

```rust
let img = Image::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `set_src(src: ImageSrc)` | Sets and retains the image source. |
| `set_offset(x: i32, y: i32)` | Shifts the image by `(x, y)` pixels relative to its normal position. |
| `set_scale(factor: u32)` | Sets the scale factor. `256` = 100% (no scaling), `128` = 50%, `512` = 200%. |
| `set_rotation(angle: i32)` | Sets the rotation in tenths of degrees (0–3600). `900` = 90°. |
| `set_pivot(x: i32, y: i32)` | Sets the pivot point for rotation and scaling, in pixels from the top-left corner. |
| `recolor(color: Color)` | Sets the tint color blended over the rendered image. Has no visible effect until `recolor_opa` is non-zero. Works on any image source including SVG (applied post-decode, preserves alpha). |
| `recolor_opa(opa: u8)` | Sets the tint strength. `0` = no tint (default), `255` = pure tint, intermediates blend. |
| `clear_recolor()` | Convenience — resets `recolor_opa` to `0` to remove the tint without losing the color. |

**Tinting images**

The `recolor` / `recolor_opa` pair is useful for swapping an icon's color on state changes without shipping multiple variants. Especially handy for SVG icons since the recolor blends over the post-decode raster while preserving alpha.

```rust
let icon = Image::new(&btn);
icon.set_src(ImageSrc::file_cstr(c"S:img/svg/lock.svg"))
    .recolor(Color::hex(0xFFFFFF))
    .recolor_opa(255); // pure white tint

// On deselect:
icon.clear_recolor(); // back to original colors
```

For a constant tint over the lifetime of the widget, use the static-style macros `image_recolor!(color_hex!(0xFFFFFF))` and `image_recolor_opa!(255)` inside a `style!(NAME, ...)` block instead of the instance setters.

**Example — C descriptor (unsafe)**

```rust
// extern "C" { static MY_LOGO: core::ffi::c_void; }
let src = unsafe { ImageSrc::descriptor(&raw const MY_LOGO) };

let img = Image::new(&screen);
img.set_src(src)
    .set_scale(256)        // 1:1
    .set_rotation(0)
    .align(LvAlign::Center, 0, 0);
```

**Example — file path (safe)**

```rust
let src = ImageSrc::file_cstr(c"/lfs/icons/logo.bin");

let img = Image::new(&container);
img.set_src(src)
    .size(Size::Px(64), Size::Px(64));
```

---

### ImageButton

An interactive button whose visual appearance per state is driven by images (`lv_imagebutton_create`). Use `set_src` to assign an image to each [`ImageButtonState`](#imagebuttonstate) you wish to customise.

This wrapper uses the **mid-section only** API — left and right sections are `NULL`. This covers most icon-button and square-button use cases without the complexity of the full 9-source stretching API.

Requires `CONFIG_LV_USE_IMAGEBUTTON=y` in Kconfig.

**Construction**

```rust
let ibtn = ImageButton::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `set_src(state: ImageButtonState, src: ImageSrc)` | Assigns and retains an image source for the given button state. Call once per state you want to customise. |

**Example — C descriptors (unsafe)**

```rust
// extern "C" { static BTN_REL: core::ffi::c_void; static BTN_PRE: core::ffi::c_void; }
let src_rel  = unsafe { ImageSrc::descriptor(&raw const BTN_REL) };
let src_pre  = unsafe { ImageSrc::descriptor(&raw const BTN_PRE) };

let ibtn = ImageButton::new(&screen);
ibtn.set_src(ImageButtonState::Released, src_rel)
    .set_src(ImageButtonState::Pressed,  src_pre)
    .size(Size::Px(80), Size::Px(80))
    .align(LvAlign::Center, 0, 0)
    .on_click(|_| { /* handle */ });
```

**Example — file paths (safe)**

```rust
let src_rel = ImageSrc::file_cstr(c"/lfs/btn/released.bin");
let src_pre = ImageSrc::file_cstr(c"/lfs/btn/pressed.bin");

let ibtn = ImageButton::new(&screen);
ibtn.set_src(ImageButtonState::Released, src_rel)
    .set_src(ImageButtonState::Pressed,  src_pre)
    .size(Size::Px(80), Size::Px(80))
    .align(LvAlign::Center, 0, 0)
    .on_click(|_| { /* handle */ });
```

---

### TextArea

A multi-line (or single-line) text input widget (`lv_textarea_create`).  When bound to a [`Keyboard`](#keyboard), the keyboard automatically types into this widget.

Requires `CONFIG_LV_USE_TEXTAREA=y` in the LVGL/Kconfig configuration.

**Construction**

```rust
let ta = TextArea::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `placeholder_text(str)` | Sets the hint text shown when the field is empty. LVGL copies the string. |
| `max_length(u32)` | Caps the maximum number of characters the user can type. `0` means unlimited. |
| `one_line(bool)` | When `true`, the widget acts as a single-line input field and ignores `\n`. |
| `password_mode(bool)` | When `true`, characters are replaced with a bullet glyph after a short delay. |
| `set_text(str)` | Overwrites the current content programmatically. LVGL copies the string. |
| `get_text() -> String` | Returns the current text content. |

**Example**

```rust
let ta = TextArea::new(&screen)
    .placeholder_text("Enter value…")
    .one_line(true)
    .max_length(32)
    .size(Size::Pct(90), Size::Px(52))
    .align(LvAlign::TopMid, 0, 20);
```

---

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
| `custom_left_slot(width)` | Enables the left segment and returns a widget handle so callers can add custom children. The handle aliases LVGL memory owned by the field; do not store or use it after the `PhoneFormatterField` is dropped. |

Invalid presets panic with clear messages: group presets require at least one
group and every group size must be greater than zero, and masks require at
least one `X` digit slot. Calling `left_slot` or `custom_left_slot` more than
once on the same field also panics. Runtime typing does not panic; the field
strips non-digits and truncates to preset capacity.

---

### Keyboard

An on-screen keyboard widget (`lv_keyboard_create`) built on LVGL's button-matrix.  The keyboard is typically docked to the bottom of the screen and bound to a [`TextArea`](#textarea).

Requires `CONFIG_LV_USE_KEYBOARD=y` in the LVGL/Kconfig configuration.

**Construction**

```rust
let kb = Keyboard::new(&parent);
```

**Methods**

| Method | Description |
|--------|-------------|
| `full_width()` | Sets width to 100 % and aligns the keyboard to the bottom-centre of its parent. The most common configuration for a docked keyboard. |
| `custom_size(w: Size, h: Size)` | Sets an explicit width and height. |
| `offset(x: i32, y: i32)` | Adjusts the keyboard position by pixel offsets from the bottom-centre anchor. |
| `layout(KeyboardLayout)` | Selects a predefined or custom layout. See [`KeyboardLayout`](#keyboardlayout). |
| `mode(LvKeyboardMode)` | Selects a layout by raw LVGL mode value. |
| `locale(KeyboardLocale)` | Convenience shorthand — picks a layout for the given locale. See [`KeyboardLocale`](#keyboardlocale). |
| `custom_map(&'static KeyMap)` | Installs a custom key map into the `User1` slot and activates it. |
| `key_style_normal(Color, impl Into<CornerRadius>)` | Sets background colour and corner radius for regular keys. |
| `key_style_action(Color)` | Sets background colour for action keys (Enter, Backspace, Space). |
| `key_style_pressed(Color)` | Sets background colour shown when any key is pressed. |
| `theme(&KeyboardTheme)` | Applies a [`KeyboardTheme`](#keyboardtheme) — background, key colours, radius, and optionally a font. |
| `bind_textarea(&TextArea)` | Binds the keyboard to a text area. Keystrokes are sent to the bound widget. |
| `on_ready(fn(Event))` | Registers a callback for `LvEventCode::Ready` (Enter / Ok key). |
| `on_cancel(fn(Event))` | Registers a callback for `LvEventCode::Cancel` (Esc / Close key). |
| `on_back(fn(Event))` | Registers a callback for the Back key. |
| `on_continue(fn(Event))` | Registers a callback for the Continue key. |
| `on_lang(fn(Event))` | Registers a callback for the language-switch (🌐) key. |
| `show()` | Makes the keyboard visible (clears `LvObjFlag::HIDDEN`). |
| `hide()` | Hides the keyboard. Can be shown again with `show()`. |
| `slide_show()` | Slides the keyboard into view with the LVGL screen-move-bottom animation. |
| `slide_hide()` | Slides the keyboard out of view with the LVGL screen-move-bottom animation. |
| `popover_keys(bool)` | Enables or disables pop-over key previews on press. |
| `preload_locale_maps(&[KeyboardLocale])` | Pre-installs the custom lowercase maps for the given locales into their LVGL user-mode slots. Subsequent `locale()` calls only need to switch the active mode. Locales without a custom map (e.g. `Numeric`) are skipped. |

**Example — full keyboard with dark theme**

```rust
use lvgl_dsl::prelude::*;

fn on_ready(_e: Event)  { /* commit input */ }
fn on_cancel(_e: Event) { /* dismiss keyboard */ }

let screen = Screen::new();

let ta = TextArea::new(&screen)
    .placeholder_text("Type here…")
    .one_line(true)
    .size(Size::Pct(90), Size::Px(52))
    .align(LvAlign::TopMid, 0, 20);

let kb = Keyboard::new(&screen)
    .full_width()
    .layout(KeyboardLayout::Qwerty)
    .theme(&KeyboardTheme::DARK)
    .popover_keys(true)
    .bind_textarea(&ta)
    .on_ready(on_ready)
    .on_cancel(on_cancel);

screen.load();
```

**Example — custom numeric map**

```rust
use lvgl_dsl::prelude::*;

let kb = Keyboard::new(&screen)
    .full_width()
    .custom_map(KEYMAP_NUMPAD)
    .theme(&KeyboardTheme::LIGHT);
```

**Example — slide animation and locale preloading**

```rust
// Pre-load locale maps for DE, FR, IT once at startup:
kb.preload_locale_maps(&[KeyboardLocale::De, KeyboardLocale::Fr, KeyboardLocale::It]);

// Show/hide with slide animation:
kb.slide_show();   // slides up from the bottom
kb.slide_hide();   // slides back down
```

> **Accent popups**: Long-pressing a letter key (e.g. `a`) displays a popup
> with accent variants (`á`, `à`, `â`, `ä`, …). Tapping a variant inserts it
> into the bound text area. This works automatically for all supported locales.

### SearchBar

A composite, callback-driven search widget (`SearchBar`) built from a text
area, clear button, result container, lazy slot containers, and optional
custom row renderer. It implements debounced input, request-token generation,
two-condition stale-reply rejection, selection with row re-rendering, load-more
pagination, initial empty/loading/error slots, footer loading/error slots, and
case-folded substring highlighting through LVGL `recolor` markup.

Unlike the chainable `Widget` widgets above, `SearchBar` is built with
`SearchBar::build(parent_ptr, SearchBarConfig)` and then driven through methods
and `FnMut` callbacks. User callbacks are dispatched after the internal
`RefCell` borrow is released, so callbacks may call back into `SearchBar`.
If callback code also uses its own `RefCell` state, avoid holding that outer
borrow across SearchBar methods that synchronously fire callbacks.

**Construction**

```rust
use lvgl_dsl::searchbar::{SearchBar, SearchBarConfig};

let mut sb = unsafe {
    SearchBar::build(parent_obj_ptr, SearchBarConfig {
        width: 400,
        height: 300,
        case_insensitive: true,
        min_query_len: 2,
        debounce_ms: 150,
    })
};
```

`SearchBar::build` is `unsafe` because it stores raw pointers into LVGL
objects that the caller must keep alive (the `parent` and any keyboard
later attached). The returned `SearchBar` owns and destroys all of its own
children when dropped, including the debounce timer.

**Configuration (`SearchBarConfig`)**

| Field | Type | Description |
|-------|------|-------------|
| `width` | `i32` | Width of the SearchBar root in LVGL pixels. |
| `height` | `i32` | Height of the root including the result list. |
| `case_insensitive` | `bool` | When `true`, text is canonicalised to lowercase before deduplication and highlight matching. |
| `min_query_len` | `usize` | Below this length the query is treated as empty (state forced to `Empty`, no `on_query_changed` fired). |
| `debounce_ms` | `u32` | Inactivity window after the last keystroke before a `QueryChanged` callback fires. |

**Callbacks (`FnMut`)**

| Method | Signature | Fires when |
|--------|-----------|------------|
| `on_query_changed(f)` | `FnMut(Token, &str)` | Debounce window elapses with a non-empty, normalised, deduplicated query. The `Token` is the request id; reply with the same token. |
| `on_query_cleared(f)` | `FnMut()` | Query becomes empty (clear button or `clear_query()`). |
| `on_select(f)` | `FnMut(u64, bool)` | A row is selected/deselected by user tap or programmatic `select`/`deselect`/`toggle_select`. Not fired for `clear_selection()` (silent). |
| `on_load_more(f)` | `FnMut(Token, u32)` | Result list scrolled near the bottom while in `Results` state. Reply with `append_results` for that token. |
| `on_retry(f)` | `FnMut(Token, &str)` | A caller-owned retry control calls the app's retry path; the callback receives the current token and query. |

**Driving the widget from your network code**

| Method | Description |
|--------|-------------|
| `set_text(s)` | Programmatically set the textarea contents (also re-canonicalises and kicks the debounce). |
| `query_text()` | Current raw textarea string. |
| `current_token()` | The token of the in-flight request. Use this when calling `set_results` / `set_loading` / `set_error`. |
| `set_results(token, rows)` → `bool` | Apply a results reply. Returns `false` if rejected by the acceptance gate (stale token or canonical mismatch); the row count then increments `stale_drop_count`. |
| `append_results(token, rows)` → `bool` | Append a load-more reply. Same gate as `set_results`. |
| `set_loading(token, on)` → `bool` | Show/hide the loading slot. `set_loading(_, false)` only checks the token (cancellation signal). |
| `set_error(token, on)` → `bool` | Show/hide error state. On `true`, stores `pre_error_state`; on `false`, restores the previous data-bearing state. |
| `clear_query()` | Programmatic clear (equivalent to user tapping the clear button). |
| `tick_debounce()` | Force-fire the debounce timer (used in tests / examples; in production the LVGL timer fires automatically). |
| `check_scroll_for_load_more()` | Manually probe the result container's scroll position. The widget also probes this on every `LV_EVENT_SCROLL_END`. |
| `request_load_more()` / `cancel_pending_load_more()` | Lower-level pagination control (most callers rely on the automatic scroll-end trigger). |
| `select(id)` / `deselect(id)` / `toggle_select(id)` | Update selection set (fires `on_select`). |
| `is_selected_id(id)` / `selected_row_ids()` / `selected_count()` | Read accessors. |
| `clear_selection()` | Silent — does NOT fire `on_select`. |
| `set_initial_empty_hint(builder)` | Builds/rebuilds the initial empty slot under the SearchBar root and syncs visibility. |
| `set_row_renderer(f)` | Installs a custom row renderer `FnMut(*mut lv_obj_t, &SearchRow, &str, bool)` and re-renders current rows. |
| `attach_keyboard(kb_ptr)` / `detach_keyboard()` | Bind/unbind an LVGL keyboard so its key presses route to the SearchBar's textarea. |
| `stale_drop_count()` | Cumulative counter of replies rejected by the acceptance gate. Useful as a telemetry signal. |
| `unsafe install_card_click(card, f)` | Free helper that attaches an LVGL clicked handler to a custom row/card object and marks it clickable. |

**Two-condition acceptance gate**

Replies are accepted only if (1) the token matches the current in-flight token AND (2) the canonical query at the time the reply arrives matches the canonical query that produced the request. Cancellation signals
(`set_loading(_, false)`, `set_error(_, false)`) only check the token.
Rejected replies bump `stale_drop_count()`.

**Slot containers (caller-filled)**

Five optional containers are lazily created on first access. The SearchBar
shows/hides them according to state; the *contents* are yours.

| Slot | How to create/fill | Visible during |
|------|--------------------|----------------|
| `initial_empty` | `set_initial_empty_hint(builder)` | Empty query / query shorter than `min_query_len`. |
| `initial_loading` | `unsafe { sb.bar.slots.ensure_initial_loading(sb.bar.root) }` | `State::Loading` with `pending_load_more == false`. |
| `initial_error` | `unsafe { sb.bar.slots.ensure_initial_error(sb.bar.root) }` | `State::Error` with `pending_load_more == false`. |
| `footer_loading` | `unsafe { sb.bar.slots.ensure_footer_loading(sb.bar.root) }` | `State::Loading` with `pending_load_more == true`. |
| `footer_error` | `unsafe { sb.bar.slots.ensure_footer_error(sb.bar.root) }` | `State::Error` with `pending_load_more == true`. |

The initial empty slot is parented to the SearchBar root, not the result
container. `render_rows()` cleans the result container on each render, so custom
row renderers should only place per-row objects under the `parent` pointer they
receive.

**SearchRow data**

Rows carry a stable `id`, required `primary` text, optional `secondary` and
`tertiary` strings, and a `disabled` flag for caller-side renderers:

```rust
SearchRow::new(1, "Stenger Thomas")
    .with_secondary("+380 50 123 45 67")
    .with_tertiary("WC-001")
    .disabled(false);
```

The default renderer highlights only `primary`. A custom renderer can display
secondary and tertiary fields however it wants.

**Highlight markup**

When `case_insensitive` is `true`, every occurrence of the canonical query
inside each row's `primary` text is wrapped with LVGL `recolor` markup
(`#FFAA00 …#` by default). Special characters in the query (`#`, `\`) are
escaped so they cannot break the markup parser. Custom renderers can call
`searchbar::highlight::highlight_markup` directly when building their own
labels.

**Example — custom recipient row renderer**

```rust
use lvgl_dsl::prelude::*;
use lvgl_dsl::searchbar::{install_card_click, SearchBar, SearchBarConfig};
use lvgl_dsl::searchbar::highlight::highlight_markup;
use lvgl_dsl::searchbar::row::SearchRow;

let mut sb = unsafe {
    SearchBar::build(parent, SearchBarConfig {
        width: 760,
        height: 360,
        case_insensitive: true,
        min_query_len: 2,
        debounce_ms: 300,
    })
};

sb.set_initial_empty_hint(|parent| unsafe {
    if parent.is_null() { return; }
    let slot = Obj::from_raw(parent);
    let label = Label::new(&slot);
    label.text("Start typing to find a recipient.");
});

sb.set_row_renderer(|parent, row, query, selected| unsafe {
    if parent.is_null() { return; }
    let parent = Obj::from_raw(parent);
    let card = Obj::new(&parent);
    card.size(Size::Pct(100), Size::Px(64))
        .bg_color(if selected { Color::hex(0xFFEFE0) } else { Color::white() })
        .border_color(Color::hex(0xC3C9CC))
        .border_width(1)
        .radius(CornerRadius::Px(8))
        .set_scrollable(false);

    let name = Label::new(&card);
    name.recolor(true)
        .text_color(Color::hex(0x1F3640))
        .text(&highlight_markup(&row.primary, query, "FF7100", true));

    if let Some(phone) = row.secondary.as_deref() {
        let secondary = Label::new(&card);
        secondary.text_color(Color::hex(0x74787A)).text(phone);
    }

    if let Some(wechip_id) = row.tertiary.as_deref() {
        let tertiary = Label::new(&card);
        tertiary.text_color(Color::hex(0x74787A)).text(wechip_id);
    }

    let row_id = row.id;
    install_card_click(card.raw_ptr() as *mut _, move || {
        // Call the app-level single-select handler here.
        log::info!("clicked row {}", row_id);
    });
});

sb.on_query_changed(|token, query| {
    log::info!("search t={} q={}", token.0, query);
});

sb.on_select(|id, on| {
    log::info!("select id={} on={}", id, on);
});

sb.on_retry(|token, query| {
    log::info!("retry t={} q={}", token.0, query);
});

// In your network reply handler:
let token = sb.current_token();
let _ = sb.set_results(token, vec![
    SearchRow::new(1, "Stenger Thomas")
        .with_secondary("+380 50 123 45 67")
        .with_tertiary("WC-001"),
]);
```

The parent project keeps an end-to-end SearchBar consumer in
`../../../apps/wechip/src/search_test.rs`.

---

### Spinner

`Spinner` wraps LVGL's `lv_spinner` widget — an animated arc that rotates
to indicate ongoing work. It inherits all `Widget` layout/style methods
and adds part-aware styling for the **track** (`LV_PART_MAIN`) and the
**indicator** (`LV_PART_INDICATOR`).

| Method | Description |
|--------|-------------|
| `Spinner::new(parent)` | Creates a spinner via `lv_spinner_create`. Panics on OOM. |
| `set_anim_params(spin_ms, arc_length_deg)` | One full rotation period and visible arc length, matching `lv_spinner_set_anim_params`. |
| `track_color(Color)` / `indicator_color(Color)` | Arc color of the background track / rotating indicator. |
| `track_width(i32)` / `indicator_width(i32)` | Arc stroke width in pixels. |
| `track_opa(u8)` / `indicator_opa(u8)` | Arc opacity (0–255). |

```rust
let s = Spinner::new(&container);
s.width(Size::Px(44))
    .height(Size::Px(44))
    .set_anim_params(900, 90)
    .track_color(Color::hex(0xE0E0E0))
    .indicator_color(Color::hex(0x4A4A4A))
    .track_width(4)
    .indicator_width(4);
```

The same widget is used as the default indicator inside
[`ButtonLoadingConfig`](#button); pass `spinner_style` there to apply
these methods without managing the spinner directly.

---

### Arc

`Arc` wraps LVGL's `lv_arc` widget. It draws an arc-shaped track with an
indicator (filled portion) and an optional draggable knob, and is the
foundation for circular progress, gauges, and countdown rings. Styling is
part-aware: the **track** (`LV_PART_MAIN`), the **indicator**
(`LV_PART_INDICATOR`), and the **knob** (`LV_PART_KNOB`) each accept their own
color, width, opacity, and rounded-cap settings.

```rust
use lvgl_dsl::prelude::*;

let arc = Arc::new(&screen);
arc
    .remove_default_style()
    .set_size(Size::Px(344), Size::Px(344))
    .align(LvAlign::Center, 0, 0)
    .set_range(0, 60)
    .set_value(60)
    .set_bg_angles(0, 360)
    .set_angles(270, 0)
    .set_rotation(0)
    .set_mode(ArcMode::Normal)
    .remove_knob()
    .track_color(Color::hex(0xE9F0F0))
    .track_width(12)
    .track_rounded(true)
    .indicator_color(Color::hex(0xE41C1C))
    .indicator_width(12)
    .indicator_rounded(true)
    .on_value_changed(|event| {
        let _arc = event.target();
    });
```

| Method | Purpose |
| --- | --- |
| `new(parent)` | Create an arc as a child of any `Widget`. |
| `set_range(min, max)` | Set the value range (defaults to LVGL's `0..100`). |
| `set_value(v)` | Set the current value; clamped to range. |
| `value()` | Read the current value as `i32`. |
| `set_bg_angles(start, end)` | Set the background arc span in degrees. |
| `set_angles(start, end)` | Set the indicator (filled) arc span in degrees. |
| `set_rotation(deg)` | Rotate the whole arc by `deg` degrees. |
| `set_mode(ArcMode)` | Choose `Normal`, `Symmetrical`, or `Reverse` fill behavior. |
| `set_change_rate(rate)` | Touch/encoder change rate (units per second). |
| `remove_default_style()` | Strip LVGL's default arc styles (calls `lv_obj_remove_style_all`); use when fully customizing. |
| `track_color(c)` / `track_width(px)` / `track_opa(opa)` / `track_rounded(bool)` | Style `LV_PART_MAIN`. |
| `indicator_color(c)` / `indicator_width(px)` / `indicator_opa(opa)` / `indicator_rounded(bool)` | Style `LV_PART_INDICATOR`. |
| `knob_color(c)` | Color the draggable thumb (`LV_PART_KNOB`). |
| `remove_knob()` | Hide the knob entirely (`bg_opa = 0` on `LV_PART_KNOB`). |
| `on_value_changed(cb)` | Install a callback fired on `LvEventCode::ValueChanged`. |

`ArcMode`:

| Variant | LVGL constant | Behavior |
| --- | --- | --- |
| `ArcMode::Normal` | `LV_ARC_MODE_NORMAL` | Indicator fills from start angle toward value. |
| `ArcMode::Symmetrical` | `LV_ARC_MODE_SYMMETRICAL` | Indicator extends symmetrically from the range midpoint. |
| `ArcMode::Reverse` | `LV_ARC_MODE_REVERSE` | Indicator fills from end angle backward. |

Use the safe [`Anim`](#anim-builder) builder to drive a smooth countdown
or progress sweep on the arc — pair the builder's per-frame callback with
`Arc::set_value()`. See [Animation](#animation) for the full API.

---

### RadioButtonList

`RadioButtonList` is a composite widget for choosing one option from a vertical
list. It creates a root container, one clickable row per option, a circular
indicator for each row, and a label for each runtime `&str` option. LVGL copies
label text when rows are created, so option labels do not need to be static C
string maps.

```rust
use lvgl_dsl::prelude::*;

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

`RadioIndicatorStyle` supports two visual patterns for the selected state:

1. **Filled circle** — set `bg_color` and `bg_opa: Some(255)` to fill the entire indicator.
2. **Ring + inner dot** — set `bg_opa: Some(0)` (transparent fill so the border becomes
   a ring) and use `dot_color` plus `dot_opa: Some(255)` to render a smaller centered
   dot inside the ring. The inner dot is a child `Obj` sized to half the indicator
   and re-centered automatically when `indicator_size()` changes.

| `RadioIndicatorStyle` field | Affects | Notes |
| --- | --- | --- |
| `bg_color`, `bg_opa` | Indicator fill | Use `bg_opa: Some(255)` for a filled circle, `bg_opa: Some(0)` for a hollow ring. |
| `border_color`, `border_width`, `border_opa` | Indicator outline | Defines the ring stroke for the ring + dot pattern. |
| `radius` | Indicator shape | Use a large value (e.g. `9999`) to keep the indicator perfectly round. |
| `dot_color`, `dot_opa` | Inner dot child | `dot_opa: Some(0)` hides the dot (default for unselected). `dot_opa: Some(255)` shows it; pair with `dot_color` for a ring + dot selected state. |

```rust
// Ring + inner dot selected state (WeCHIP "What went wrong?" Figma 623:35967)
radio
    .indicator_size(34)
    .indicator_style(RadioIndicatorStyle {
        bg_opa: Some(0),
        border_color: Some(Color::hex(0x1F3640)),
        border_width: Some(2),
        radius: Some(9999),
        dot_color: None,
        dot_opa: Some(0),
        ..Default::default()
    })
    .selected_indicator_style(RadioIndicatorStyle {
        bg_opa: Some(0),
        border_color: Some(Color::hex(0xFF7100)),
        border_width: Some(2),
        radius: Some(9999),
        dot_color: Some(Color::hex(0xFF7100)),
        dot_opa: Some(255),
        ..Default::default()
    });
```

---

## Animation

`Anim` is a safe builder around LVGL's `lv_anim_*` API. It replaces hand-written
`unsafe { lv_anim_init(...); lv_anim_set_*(...); lv_anim_start(...) }` blocks
with a chainable Rust API, manages the underlying `lv_anim_t` lifecycle via a
`Drop`-based handle, and supports both **closures** (desktop / tests) and
**raw `extern "C" fn` pointers** (works on `no_std` Zephyr targets).

### Anim Builder

Create an animation with `Anim::new(var)`, where `var` is the stable pointer
used to identify the animation in LVGL (typically the target object pointer,
or any address that lives as long as the animation does). Chain setters, then
call `.start()`.

```rust
use lvgl_dsl::prelude::*;
use core::ffi::c_void;

let handle = Anim::new(arc.raw_ptr() as *mut c_void)
    .values(duration_ms as i32, 0)
    .duration_ms(duration_ms)
    .path(AnimPath::Linear)
    .exec_extern(arc_set_value_trampoline)     // fn pointer — no_std safe
    .completed_extern(on_countdown_done)
    .start();
```

| Method | Description |
| --- | --- |
| `Anim::new(var: *mut c_void) -> Anim` | Start a builder with the given identity pointer. |
| `.values(start: i32, end: i32)` | Animate the i32 value from `start` to `end`. Maps to `lv_anim_set_values`. |
| `.duration_ms(ms: u32)` | Total animation duration. Maps to `lv_anim_set_duration`. |
| `.path(p: AnimPath)` | Easing curve. See [Path](#path). |
| `.repeat_count(n: u32)` | Number of repetitions. Use `lvgl_dsl::LV_ANIM_REPEAT_INFINITE` for infinite loops. |
| `.exec(closure)` | **Desktop / test only.** Per-frame callback `FnMut(*mut c_void, i32)`. Closures are stored in a slot map keyed by `var`. |
| `.on_completed(closure)` | **Desktop / test only.** Completion callback `FnMut(*mut c_void)`. The slot is freed after firing. |
| `.exec_extern(f: unsafe extern "C" fn(*mut c_void, i32))` | Raw fn-pointer per-frame callback. Works on **all** targets including `no_std` Zephyr. Required when the callback must live on a real device. |
| `.completed_extern(f: unsafe extern "C" fn(*mut lv_anim_t))` | Raw fn-pointer completion callback. Works on **all** targets. |
| `.start() -> AnimHandle` | Launch the animation. The returned handle owns it (see below). |

> **Closures vs externs.** If both `.exec(...)` and `.exec_extern(...)` are
> set on the same builder, the extern fn pointer wins and the closure is
> dropped. Same rule for `.on_completed` vs `.completed_extern`.

> **`no_std` Zephyr builds.** `.exec` and `.on_completed` are compiled out
> on Zephyr because they require `Box<dyn FnMut>`. Use `.exec_extern` and
> `.completed_extern` to support both desktop and device.

### AnimHandle

`Anim::start()` returns an `AnimHandle`. Dropping the handle calls
`lv_anim_delete(var, exec_cb)` and (for closure-based animations) clears the
slot from the internal map — releasing any boxed closures.

```rust
struct MyView {
    anim_handle: Option<AnimHandle>,
}

impl MyView {
    fn start(&mut self) {
        self.anim_handle = Some(
            Anim::new(self.anim_token.get().cast::<c_void>())
                .values(30_000, 0)
                .duration_ms(30_000)
                .exec_extern(my_exec)
                .completed_extern(my_done)
                .start(),
        );
    }

    fn stop(&mut self) {
        // RAII: drop cancels the animation automatically.
        self.anim_handle = None;
    }
}
```

For fire-and-forget animations, prefer **`Anim::start_detached()`**, which never
returns a handle and so cannot be leaked:

```rust
Anim::new(target).duration_ms(800).start_detached();
```

`core::mem::forget(handle)` works, but only safely with **extern-only**
animations (`.exec_extern(...)` / `.completed_extern(...)`). With closure
forms (`.exec(|…|)` / `.on_completed(|…|)`), forgetting the handle prevents
slot-table cleanup on drop and can leak the boxed closures indefinitely —
especially for `LV_ANIM_REPEAT_INFINITE`, where no completion callback ever
fires to reclaim the slot. Use `start_detached()` for fire-and-forget, or
keep the handle for the animation's lifetime.

### Path

`AnimPath` (re-exported from `Path`) selects the easing curve. Each variant
maps directly to LVGL's built-in `lv_anim_path_*` function. Use `Custom(...)`
to plug in your own.

| Variant | LVGL function | Behavior |
| --- | --- | --- |
| `AnimPath::Linear` | `lv_anim_path_linear` | Constant rate. |
| `AnimPath::EaseIn` | `lv_anim_path_ease_in` | Slow start, fast finish. |
| `AnimPath::EaseOut` | `lv_anim_path_ease_out` | Fast start, slow finish. |
| `AnimPath::EaseInOut` | `lv_anim_path_ease_in_out` | Slow at both ends. |
| `AnimPath::Overshoot` | `lv_anim_path_overshoot` | Overshoots, then settles. |
| `AnimPath::Bounce` | `lv_anim_path_bounce` | Bounces near the end. |
| `AnimPath::Step` | `lv_anim_path_step` | Jumps from start to end at completion. |
| `AnimPath::Custom(fn)` | user-supplied | `unsafe extern "C" fn(*const lv_anim_t) -> i32`. |

---

Every widget (`Screen`, `Obj`, `Button`, `ButtonMatrix`, `ParcelLocker`, `Label`, `Dropdown`, `QrCode`, `Image`, `ImageButton`, `TextArea`, `PhoneFormatterField`, `Keyboard`, `SearchBar`, `Arc`, `RadioButtonList`) implements the `Widget` trait. All methods return `&Self`, enabling method chaining.

### Alignment & Position

| Method | Description |
|--------|-------------|
| `align(align: LvAlign, x_ofs: i32, y_ofs: i32)` | Positions the widget relative to its parent using the specified alignment anchor, with optional pixel offsets. |

```rust
widget.align(LvAlign::Center, 0, 0);
widget.align(LvAlign::TopRight, -10, 10);
```

---

### Flexbox Layout

| Method | Description |
|--------|-------------|
| `flex_row()` | Arranges children in a horizontal row (`FlexFlow::Row`). |
| `flex_col()` | Arranges children in a vertical column (`FlexFlow::Column`). |
| `flex_row_wrap()` | Horizontal row with wrapping (`FlexFlow::RowWrap`). |
| `set_flex_flow(FlexFlow)` | Sets any `FlexFlow` variant directly. |
| `flex_align(main, cross, track)` | Controls main-axis, cross-axis, and track alignment using `FlexAlign` values. |
| `flex_grow(u8)` | Sets the flex-grow factor for this widget within its flex parent. |
| `gap(px: i32)` | Sets both row and column gap between children. |
| `pad_all(px: i32)` | Sets equal padding on all four sides. |
| `pad_left(px: i32)` | Sets left padding only. |
| `pad_right(px: i32)` | Sets right padding only. |
| `pad_top(px: i32)` | Sets top padding only. |
| `pad_bottom(px: i32)` | Sets bottom padding only. |

```rust
container
    .flex_row()
    .flex_align(FlexAlign::SpaceBetween, FlexAlign::Center, FlexAlign::Start)
    .gap(8)
    .pad_all(12);
```

---

### Sizing

| Method | Description |
|--------|-------------|
| `width(Size)` | Sets the width. |
| `height(Size)` | Sets the height. |
| `size(w: Size, h: Size)` | Sets width and height in one call. |
| `min_width(Size)` | Sets the minimum width. |
| `max_width(Size)` | Sets the maximum width. |
| `min_height(Size)` | Sets the minimum height. |
| `max_height(Size)` | Sets the maximum height. |

```rust
widget.size(Size::Pct(100), Size::Px(60));
widget.min_width(Size::Px(80)).max_width(Size::Pct(50));
```

---

### Background

| Method | Description |
|--------|-------------|
| `bg_color(Color)` | Sets the background fill colour. |
| `bg_opa(u8)` | Sets background opacity (`0` = transparent, `255` = opaque). |

---

### Background Image

Overlays an image on the widget's background, drawn on top of `bg_color`.

| Method | Description |
|--------|-------------|
| `bg_image(&ImageSrc)` | Sets the background image source. The source must outlive the widget. |
| `bg_image_tiled(bool)` | When `true`, tiles the image to fill the widget area. |
| `bg_image_opa(u8)` | Sets the background image opacity (`0`–`255`). |
| `bg_image_recolor(Color)` | Tints the background image with `color`. |
| `bg_image_recolor_opa(u8)` | Controls the recolor tint intensity (`0` = no tint, `255` = solid color). |

**C descriptor (unsafe)**

```rust
// extern "C" { static BG_TEXTURE: core::ffi::c_void; }
let src = unsafe { ImageSrc::descriptor(&raw const BG_TEXTURE) };

let panel = Obj::new(&screen)
    .size(Size::Pct(100), Size::Pct(100))
    .bg_image(&src)
    .bg_image_tiled(true)
    .bg_image_opa(200);
```

**File path (safe)**

```rust
let src = ImageSrc::file_cstr(c"/lfs/textures/bg.bin");

let panel = Obj::new(&screen)
    .size(Size::Pct(100), Size::Pct(100))
    .bg_image(&src)
    .bg_image_tiled(true)
    .bg_image_opa(200);
```

---

### Text

| Method | Description |
|--------|-------------|
| `text_color(Color)` | Sets the text colour (applies to any child labels). |
| `text_opa(u8)` | Sets text opacity. |
| `text_font(&Font)` | Sets the font for text within this widget. |

---

### Shape & Opacity

| Method | Description |
|--------|-------------|
| `radius(impl Into<CornerRadius>)` | Sets a uniform corner radius. Accepts `CornerRadius` variants or a plain `i32`. |
| `opacity(u8)` | Sets the overall opacity of the widget (`0`–`255`). |

```rust
widget.radius(CornerRadius::Full);  // pill shape
widget.radius(8);                   // 8 px — i32 auto-converts
widget.opacity(180);
```

---

### Border

| Method | Description |
|--------|-------------|
| `border_color(Color)` | Sets the border colour. |
| `border_width(i32)` | Sets the border thickness in pixels. |
| `border_opa(u8)` | Sets border opacity. |
| `border_side(BorderSide)` | Selects which sides draw a border. Sides can be combined with `\|`. |

```rust
widget
    .border_color(Color::hex(0x888888))
    .border_width(2)
    .border_side(BorderSide::BOTTOM | BorderSide::TOP);
```

---

### Outline

An outer glow/ring that sits outside the widget's border and does not affect layout.

| Method | Description |
|--------|-------------|
| `outline_color(Color)` | Sets the outline colour. |
| `outline_width(i32)` | Sets the outline thickness in pixels. |
| `outline_opa(u8)` | Sets outline opacity. |
| `outline_pad(i32)` | Sets the gap between the widget border and the outline. |

---

### Shadow

| Method | Description |
|--------|-------------|
| `shadow_color(Color)` | Sets the shadow colour. |
| `shadow_width(i32)` | Sets the shadow blur width. |
| `shadow_opa(u8)` | Sets shadow opacity. |
| `shadow_offset(x: i32, y: i32)` | Sets the shadow's X and Y offset. |
| `shadow_spread(i32)` | Expands or contracts the shadow beyond the widget boundary. |

```rust
widget
    .shadow_color(Color::black())
    .shadow_width(20)
    .shadow_opa(128)
    .shadow_offset(4, 4);
```

---

### Events

| Method | Description |
|--------|-------------|
| `on_event(cb: fn(Event), code: LvEventCode)` | Registers a callback for a specific event code. |
| `on_click(cb: fn(Event))` | Convenience — registers a callback for `LvEventCode::Clicked`. |

Callbacks are plain function pointers (`fn(Event)`). Multiple callbacks can be registered on the same widget.

```rust
fn handle_click(e: Event) {
    // use e.code() or e.target()
}

widget.on_click(handle_click);
widget.on_event(handle_click, LvEventCode::Focused);
```

---

### State

State is a bitfield (`LvState`). States can be combined with `|`.

| Method | Description |
|--------|-------------|
| `add_state(LvState)` | Applies one or more states. |
| `remove_state(LvState)` | Clears one or more states. |
| `has_state(LvState) -> bool` | Returns `true` if all given state bits are set. |

---

### Flags

Flags control widget behaviour. They are a bitfield (`LvObjFlag`) and can be combined with `|`.

| Method | Description |
|--------|-------------|
| `add_flag(LvObjFlag)` | Sets one or more behaviour flags. |
| `remove_flag(LvObjFlag)` | Clears one or more flags. |
| `has_flag(LvObjFlag) -> bool` | Returns `true` if all given flag bits are set. |

```rust
let overlay = Obj::new(&root);
overlay
    .align(LvAlign::BottomMid, 0, -8)
    .size(Size::Pct(100), Size::Px(48))
    .add_flag(LvObjFlag::IGNORE_LAYOUT | LvObjFlag::FLOATING)
    .set_hidden(true);
```

---

### Convenience Setters/Getters

Sugar methods that internally delegate to state/flag operations.

| Method | Description |
|--------|-------------|
| `set_hidden(bool)` | Shows or hides the widget (`LvObjFlag::HIDDEN`). |
| `set_disabled(bool)` | Enables or disables the widget (`LvState::DISABLED`). |
| `set_checked(bool)` | Sets or clears the checked state (`LvState::CHECKED`). |
| `set_checkable(bool)` | Enables toggle behaviour on click (`LvObjFlag::CHECKABLE`). |
| `set_clickable(bool)` | Makes the widget respond to pointer events (`LvObjFlag::CLICKABLE`). |
| `set_scrollable(bool)` | Enables or disables scrolling (`LvObjFlag::SCROLLABLE`). |
| `is_hidden() -> bool` | Returns whether the widget is hidden. |
| `is_disabled() -> bool` | Returns whether the widget is disabled. |
| `is_checked() -> bool` | Returns whether the widget is checked. |

---

### Raw Pointer Access

| Method | Description |
|--------|-------------|
| `raw_ptr() -> usize` | Returns the raw `lv_obj_t *` pointer as a `usize`. Useful for storing widget handles across FFI boundaries (e.g. `AtomicUsize`). Only meaningful while the LVGL object is alive. |

---

### Cleanup

| Method | Description |
|--------|-------------|
| `delete(self)` | Consumes the widget handle and removes the object from LVGL. Prevents use-after-free by requiring ownership. |

---

## Supporting Types

### Color

```rust
Color::hex(0xFF8800)               // 24-bit hex
Color::rgb(255, 136, 0)            // R, G, B components
Color::white()
Color::black()
Color::palette(Palette::Blue)      // main palette swatch
Color::palette_light(Palette::Red, 3)  // lighter shade (level 1–5)
Color::palette_dark(Palette::Red, 2)   // darker shade (level 1–5)
```

---

### Palette

19 predefined material-design palettes:

| Variant | Variant | Variant |
|---------|---------|---------|
| `Red` | `Pink` | `Purple` |
| `DeepPurple` | `Indigo` | `Blue` |
| `LightBlue` | `Cyan` | `Teal` |
| `Green` | `LightGreen` | `Lime` |
| `Yellow` | `Amber` | `Orange` |
| `DeepOrange` | `Brown` | `BlueGrey` |
| `Grey` | | |

---

### Size

Used by all sizing methods.

| Variant | Description |
|---------|-------------|
| `Size::Px(i32)` | Explicit pixel value. |
| `Size::Pct(i32)` | Percentage of the parent's size. |
| `Size::Content` | Shrink-wraps to the widget's content. |

```rust
widget.size(Size::Pct(100), Size::Content);
widget.width(Size::Px(240));
```

---

### LvAlign

Controls positioning anchor in `align()`. Inner alignments are relative to the parent; `Out*` alignments place the widget outside the parent.

| Inner | Outer |
|-------|-------|
| `Default`, `TopLeft`, `TopMid`, `TopRight` | `OutTopLeft`, `OutTopMid`, `OutTopRight` |
| `BottomLeft`, `BottomMid`, `BottomRight` | `OutBottomLeft`, `OutBottomMid`, `OutBottomRight` |
| `LeftMid`, `RightMid`, `Center` | `OutLeftTop`, `OutLeftMid`, `OutLeftBottom` |
| | `OutRightTop`, `OutRightMid`, `OutRightBottom` |

---

### FlexFlow

Controls the direction and wrapping behaviour of a flex container.

| Variant | Description |
|---------|-------------|
| `Row` | Left-to-right, no wrap. |
| `Column` | Top-to-bottom, no wrap. |
| `RowWrap` | Left-to-right with wrapping. |
| `ColumnWrap` | Top-to-bottom with wrapping. |
| `RowReverse` | Right-to-left, no wrap. |
| `ColumnReverse` | Bottom-to-top, no wrap. |
| `RowWrapReverse` | Right-to-left with wrapping. |
| `ColumnWrapReverse` | Bottom-to-top with wrapping. |

---

### FlexAlign

Controls how children are distributed within a flex container. Used for the `main`, `cross`, and `track` arguments of `flex_align()`.

| Variant | Description |
|---------|-------------|
| `Start` | Pack towards the start of the axis. |
| `End` | Pack towards the end of the axis. |
| `Center` | Centre on the axis. |
| `SpaceEvenly` | Equal gaps including before first and after last. |
| `SpaceAround` | Half-size gaps at the start and end. |
| `SpaceBetween` | No gap at start/end; equal gaps between items. |

---

### CornerRadius

Used by `radius()`. Also accepts a plain `i32`, which is treated as `Px(n)`.

| Variant | Description |
|---------|-------------|
| `CornerRadius::None` | No rounding (radius = 0). |
| `CornerRadius::Full` | Fully rounded — pill or circle shape (`LV_RADIUS_CIRCLE`). |
| `CornerRadius::Px(i32)` | Custom pixel radius, applied uniformly to all four corners. |

> LVGL 9.x applies a single uniform radius to all corners. Per-corner control is not available.

---

### BorderSide

A bitfield — combine sides with `|`.

| Constant | Description |
|----------|-------------|
| `BorderSide::NONE` | No border. |
| `BorderSide::TOP` | Top edge only. |
| `BorderSide::BOTTOM` | Bottom edge only. |
| `BorderSide::LEFT` | Left edge only. |
| `BorderSide::RIGHT` | Right edge only. |
| `BorderSide::FULL` | All four sides. |
| `BorderSide::INTERNAL` | Inner border between children (used with lists). |

```rust
widget.border_side(BorderSide::TOP | BorderSide::BOTTOM);
```

---

### LvDropdownDir

Controls the direction in which a [`Dropdown`](#dropdown) list opens.

| Variant | Value | Description |
|---------|-------|-------------|
| `LvDropdownDir::Down` | 0 | Open below the button (default). |
| `LvDropdownDir::Up` | 1 | Open above the button. |
| `LvDropdownDir::Left` | 2 | Open to the left. |
| `LvDropdownDir::Right` | 3 | Open to the right. |

```rust
dd.direction(LvDropdownDir::Up);
```

---

### Font

Wraps a static LVGL font symbol. Pass a reference to `text_font()`.

| Constructor | Description |
|-------------|-------------|
| `Font::montserrat_14()` | Montserrat, 14 px. Requires the font to be enabled in LVGL config. |
| `Font::montserrat_30()` | Montserrat, 30 px. Requires the font to be enabled in LVGL config. |

```rust
label.text_font(&Font::montserrat_30());
```

---

### ImageSrc

A source pointer for LVGL image widgets. Can reference a C array descriptor or a file-system path.

LVGL stores the pointer directly — the pointed-to data must remain valid for the lifetime of any widget using this source.

| Constructor | Safety | Description |
|-------------|--------|-------------|
| `unsafe ImageSrc::descriptor(ptr: *const c_void)` | `unsafe` | Wraps a pointer to a `const lv_image_dsc_t` descriptor generated by LVGL's image converter tool. |
| `ImageSrc::file(path: &str)` | safe | Wraps a file-system path (e.g. LittleFS or FAT). Returns an error if the path contains an interior NUL byte. |
| `ImageSrc::file_cstr(path: &'static CStr)` | safe | Wraps a static C-string file-system path literal. |

```rust
// C array descriptor (generated by LVGL image converter)
extern "C" { static MY_ICON: core::ffi::c_void; }
let src = unsafe { ImageSrc::descriptor(&raw const MY_ICON) };

// File-system path
let src = ImageSrc::file("S:icons/logo.bin")?;
let static_src = ImageSrc::file_cstr(c"S:icons/logo.bin");
```

---

### ImageButtonState

Selects which visual state an image source is assigned to in [`ImageButton::set_src`](#imagebutton).

| Variant | Value | Description |
|---------|-------|-------------|
| `Released` | 0 | Normal, un-pressed state. |
| `Pressed` | 1 | Actively pressed. |
| `Disabled` | 2 | Widget is disabled (`LvState::DISABLED`). |
| `CheckedReleased` | 3 | Checked / toggled, un-pressed. |
| `CheckedPressed` | 4 | Checked and pressed. |
| `CheckedDisabled` | 5 | Checked and disabled. |

```rust
ibtn.set_src(ImageButtonState::Released, src_normal)
    .set_src(ImageButtonState::Pressed,  src_pressed);
```

---

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

---

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

---

### LvState

A bitfield representing the current interaction state of a widget. States can be combined with `|`.

| Constant | Description |
|----------|-------------|
| `LvState::DEFAULT` | No special state. |
| `LvState::CHECKED` | Widget is toggled/selected. |
| `LvState::FOCUSED` | Widget has focus (pointer). |
| `LvState::FOCUS_KEY` | Widget has focus (keyboard/encoder). |
| `LvState::EDITED` | Value is being edited by an encoder. |
| `LvState::HOVERED` | Pointer is over the widget but not pressing. |
| `LvState::PRESSED` | Widget is currently being pressed. |
| `LvState::SCROLLED` | Widget is being scrolled. |
| `LvState::DISABLED` | Widget is disabled (no interaction). |

---

### LvObjFlag

A bitfield controlling widget behaviour. Flags can be combined with `|`.

| Constant | Description |
|----------|-------------|
| `LvObjFlag::HIDDEN` | Object is not rendered. |
| `LvObjFlag::CLICKABLE` | Object receives pointer click events. |
| `LvObjFlag::CLICK_FOCUSABLE` | Clicking focuses the widget. |
| `LvObjFlag::CHECKABLE` | Clicks toggle `LvState::CHECKED`. |
| `LvObjFlag::SCROLLABLE` | Object can scroll its content. |
| `LvObjFlag::SCROLL_ELASTIC` | Scroll bounces at the edges. |
| `LvObjFlag::SCROLL_MOMENTUM` | Scroll has inertia after release. |
| `LvObjFlag::EVENT_BUBBLE` | Events propagate to the parent. |
| `LvObjFlag::IGNORE_LAYOUT` | Parent flex/grid layout ignores this object. Useful for overlays. |
| `LvObjFlag::FLOATING` | Object does not scroll with parent content and can float above layout. |

---

### Event

Passed to event callbacks. Provides information about the triggering event.

| Method | Description |
|--------|-------------|
| `code() -> LvEventCode` | The event type that fired. |
| `target() -> LvObj` | The widget that received the event. |
| `target_raw_ptr() -> usize` | Returns the raw target-object pointer as a `usize`. Use inside static `fn(Event)` callbacks to pass the target across FFI boundaries. |

```rust
fn my_handler(e: Event) {
    match e.code() {
        LvEventCode::Clicked => { /* ... */ }
        LvEventCode::Focused => { /* ... */ }
        _ => {}
    }
}
```

---

### LvEventCode

| Variant | Description |
|---------|-------------|
| `All` | Wildcard — fires for all events. |
| `Pressed` | Pointer/key pressed down. |
| `ShortClicked` | Released quickly without long press. |
| `LongPressed` | Pointer/key held down past the long-press threshold. |
| `Clicked` | Pressed and released on the widget. |
| `Released` | Pointer/key released. |
| `Focused` | Widget gained focus. |
| `Defocused` | Widget lost focus. |
| `ValueChanged` | Widget value changed (sliders, checkboxes). |
| `Ready` | Keyboard Enter / Ok key was pressed. |
| `Cancel` | Keyboard Esc / Close key was pressed. |
| `ScreenLoadStart` | Screen load animation started. |
| `ScreenLoaded` | Screen is fully loaded and active. |
| `ScreenUnloaded` | Screen has been replaced and hidden. |

---

### ScreenAnim

Used by `Screen::load_anim()`.

| Variant | Description |
|---------|-------------|
| `None` | Instant switch, no animation. |
| `OverLeft` | New screen slides in over the old one from the right. |
| `OverRight` | New screen slides in from the left. |
| `OverTop` | New screen slides in from the bottom. |
| `OverBottom` | New screen slides in from the top. |
| `MoveLeft` | Both screens move left (old exits, new enters). |
| `MoveRight` | Both screens move right. |
| `MoveTop` | Both screens move up. |
| `MoveBottom` | Both screens move down. |
| `FadeIn` | New screen fades in over the old. |
| `FadeOut` | Old screen fades out, revealing the new. |
| `OutLeft` | Old screen exits to the left; new screen is already in place. |
| `OutRight` | Old screen exits to the right. |
| `OutTop` | Old screen exits upward. |
| `OutBottom` | Old screen exits downward. |

```rust
screen.load_anim(ScreenAnim::MoveLeft, 400, 0, true);
```

---

### KeyboardLayout

Selects the active keyboard layout. Most variants map to an LVGL built-in mode; `Custom` loads a user-provided [`KeyMap`](#keymap--keymapentry) into the `User1` slot.

| Variant | LVGL mode | Description |
|---------|-----------|-------------|
| `Qwerty` | `TextLower` | Lower-case QWERTY (built-in). |
| `QwertyUpper` | `TextUpper` | Upper-case QWERTY (built-in). |
| `NumberPad` | `Number` | Numeric pad (built-in). |
| `SpecialChars` | `Special` | Special-character layout (built-in). |
| `Locale(KeyboardLocale)` | varies | Selects the layout and LVGL slot automatically for a language locale. Prefer this over raw `Qwerty`/`NumberPad` when targeting a specific language. See [`KeyboardLocale`](#keyboardlocale). |
| `Custom(&'static KeyMap)` | `User1` | Fully custom layout. Calling `layout(Custom(map))` installs `map` into the LVGL `User1` slot and activates it. |

```rust
kb.layout(KeyboardLayout::Qwerty);
kb.layout(KeyboardLayout::Locale(KeyboardLocale::De));
kb.layout(KeyboardLayout::Custom(KEYMAP_NUMPAD));
```

---

### LvKeyboardMode

A direct passthrough to the `lv_keyboard_mode_t` C enum.  Prefer [`KeyboardLayout`](#keyboardlayout) for the DSL-friendly API.

| Variant | Value | Description |
|---------|-------|-------------|
| `TextLower` | 0 | Lower-case text. |
| `TextUpper` | 1 | Upper-case text. |
| `Special` | 2 | Special characters. |
| `Number` | 3 | Numeric pad. |
| `User1` | 4 | User-defined slot 1 — used for `KeyboardLayout::Custom`. |
| `User2` | 5 | User-defined slot 2 — used for `KeyboardLocale::De`. |
| `User3` | 6 | User-defined slot 3 — used for `KeyboardLocale::Fr`. |
| `User4` | 7 | User-defined slot 4 — used for `KeyboardLocale::It`. |
| `User5` | 8 | User-defined slot 5 — used for `KeyboardLocale::FrCh`. |
| `User6` | 9 | User-defined slot 6 — used for `KeyboardLocale::Ua`. |

---

### KeyboardLocale

A convenience shorthand that maps a locale to a [`KeyboardLayout`](#keyboardlayout) via `From<KeyboardLocale>`.  Each locale selects an LVGL mode slot and installs a custom key map into that slot (except `Numeric` which uses the LVGL built-in Number pad).

| Variant | Maps to | LVGL mode slot | Custom map? |
|---------|---------|----------------|-------------|
| `KeyboardLocale::EnUs` | `KeyboardLayout::Locale(EnUs)` | `TextLower` (0) | Yes — custom QWERTY English lower/upper maps |
| `KeyboardLocale::Numeric` | `KeyboardLayout::Locale(Numeric)` | `Number` (3) | No — LVGL built-in |
| `KeyboardLocale::De` | `KeyboardLayout::Locale(De)` | `User2` (5) | Yes — QWERTZ with ü/ö/ä/ß |
| `KeyboardLocale::Fr` | `KeyboardLayout::Locale(Fr)` | `User3` (6) | Yes — AZERTY with é/è/à/ç |
| `KeyboardLocale::It` | `KeyboardLayout::Locale(It)` | `User4` (7) | Yes — QWERTY with à/è/ì/ò/ù |
| `KeyboardLocale::FrCh` | `KeyboardLayout::Locale(FrCh)` | `User5` (8) | Yes — QWERTZ with é/è/à/ç |
| `KeyboardLocale::Ua` | `KeyboardLayout::Locale(Ua)` | `User6` (9) | Yes — Cyrillic ЙЦУКЕН |

```rust
kb.locale(KeyboardLocale::EnUs);
```

This enum is `#[non_exhaustive]` — new locales may be added without breaking existing code.

---

### KeyMap / KeyMapEntry

`type KeyMap = [KeyMapEntry]` — a flat slice of C-string pointers describing one keyboard layout.

`KeyMapEntry` is a `#[repr(transparent)]` newtype over `*const c_char` with `unsafe impl Sync`, which allows the map to be stored in `static` context.  Construct entries with `KeyMapEntry::new(c"...")`.

**Layout rules (LVGL convention):**
- Each entry is a `KeyMapEntry::new(c"label")` pointing to a key label.
- `KeyMapEntry::new(c"\n")` ends the current row and starts a new one.
- `KeyMapEntry::new(c"")` terminates the entire map (required last element).

Predefined maps available in the prelude:

| Name | Description |
|------|-------------|
| `KEYMAP_QWERTY_EN` | 4-row English QWERTY (q-p / a-l / ABC+z-m+Del / #@!+space+Ok). |
| `KEYMAP_QWERTY_EN_LC` / `_UC` | English lower-case and upper-case maps with action keys (Del, Back, Continue) and a locale-label lang key (e.g. `EN`). |
| `KEYMAP_QWERTY_DE` / `_LC` / `_UC` | German QWERTZ maps (standard, lower-case, upper-case). |
| `KEYMAP_QWERTY_FR` / `_LC` / `_UC` | French AZERTY maps. |
| `KEYMAP_QWERTY_IT` / `_LC` / `_UC` | Italian QWERTY maps with accent row. |
| `KEYMAP_QWERTY_FRCH_LC` / `_UC` | Swiss French QWERTZ maps. |
| `KEYMAP_UA_LC` / `_UC` | Ukrainian ЙЦУКЕН maps. |
| `KEYMAP_NUMPAD` | Numeric pad (1–9 / Del+0+Ok rows). |

Each locale map has a corresponding **ctrl map** (`CTRLMAP_QWERTY_EN`, `CTRLMAP_QWERTY_EN_LC`, etc.) that controls per-key widths and action-key styling.

**Custom map example:**

```rust
use lvgl_dsl::prelude::*;

static MY_MAP: &KeyMap = &[
    KeyMapEntry::new(c"A"), KeyMapEntry::new(c"B"), KeyMapEntry::new(c"C"),
    KeyMapEntry::new(c"\n"),
    KeyMapEntry::new(c"Del"), KeyMapEntry::new(c"Ok"),
    KeyMapEntry::new(c""),  // terminator
];

kb.custom_map(MY_MAP);
```

---

### CtrlMap

`type CtrlMap = [u32]` — a parallel companion to [`KeyMap`](#keymap--keymapentry). One `u32` entry per actual key button (excluding `\n` row-separators and the `""` terminator).

Each entry encodes:
- **Bits 0–2** — relative button width (1–7 units). Widths are distributed proportionally within each row.
- **Bit 4** — `CTRL_HIDDEN` (`0x0010`): render the button as an invisible spacer.
- **Bit 8** — `CTRL_CHECKED` (`0x0100`): marks action keys (ABC, Del, Back, etc.) for separate styling via `key_style_action()`.

**Width constants:**

| Constant | Value | Description |
|----------|-------|-------------|
| `CTRL_W1` | 1 | Standard letter key (1 unit). |
| `CTRL_W2` | 2 | Action keys (ABC, Del). |
| `CTRL_W3` | 3 | Bottom-row keys (Ok, #@!). |
| `CTRL_W4` | 4 | Wider bottom-row keys (Back, Continue). |
| `CTRL_W5` | 5 | 5 units. |
| `CTRL_W6` | 6 | Extended space bar. |
| `CTRL_SPACE_W` | 7 | Comfortable space-bar width. |
| `CTRL_HIDDEN` | `0x0010` | Invisible spacer key. |
| `CTRL_CHECKED` | `0x0100` | Marks action keys for `LV_STATE_CHECKED` styling. |
| `CTRL_SPACER` | `CTRL_HIDDEN \| CTRL_W1` | Hidden 1-unit spacer (row indent). |

The array length **must** equal the number of non-`\n`, non-`""` entries in the paired [`KeyMap`](#keymap--keymapentry); a mismatch causes undefined LVGL behaviour.

---

### KeyboardTheme

A struct describing the visual styling of a [`Keyboard`](#keyboard): background colour, key colours, corner radius, and an optional font override.

```rust
pub struct KeyboardTheme {
    pub bg_hex:          u32,
    pub key_normal_hex:  u32,
    pub key_action_hex:  u32,
    pub key_radius_px:   i32,
    pub font:            Option<fn() -> Font>,
}
```

Fields store raw `u32` hex values (not `Color`) so that the struct can be a `const`.  Conversion to `Color` happens when `theme()` is called.

**Predefined constants:**

| Constant | `bg_hex` | `key_normal_hex` | `key_action_hex` | `key_radius_px` |
|----------|----------|-----------------|-----------------|----------------|
| `KeyboardTheme::LIGHT` | `0xF5F5F5` | `0xFFFFFF` | `0xE0E0E0` | 6 |
| `KeyboardTheme::DARK` | `0x1E1E1E` | `0x333333` | `0x555555` | 6 |

**Custom theme example:**

```rust
const MY_THEME: KeyboardTheme = KeyboardTheme {
    bg_hex:         0x101820,
    key_normal_hex: 0x1E2D3D,
    key_action_hex: 0x2E8B57,
    key_radius_px:  10,
    font:           None,
};

kb.theme(&MY_THEME);
```

---

### LocaleSwitcher

A zero-allocation, `no_std`-safe cursor that cycles through a fixed set of [`KeyboardLocale`](#keyboardlocale) values. Const-constructible and suitable for `static` storage in embedded firmware.

```rust
use lvgl_dsl::prelude::*;

static LOCALES: LocaleSwitcher<3> = LocaleSwitcher::new([
    KeyboardLocale::EnUs,
    KeyboardLocale::De,
    KeyboardLocale::Fr,
]);
```

**Methods**

| Method | Description |
|--------|-------------|
| `const new(locales: [KeyboardLocale; N]) -> Self` | Creates a new switcher. Panics if `N == 0`. |
| `current() -> KeyboardLocale` | Returns the active locale without advancing. |
| `next() -> KeyboardLocale` | Advances to the next locale (wraps after last) and returns it. |
| `set(locale) -> bool` | Sets the cursor to `locale` if present; returns `false` otherwise. |
| `index() -> usize` | Returns the current zero-based cursor position. |
| `len() -> usize` | Returns the number of locales (`N`). |
| `is_empty() -> bool` | Always `false` (N > 0 is enforced). |

**Example — cycle on language-key press:**

```rust
fn on_lang_button(kb: &Keyboard, sw: &mut LocaleSwitcher<3>) {
    let next = sw.next();
    kb.locale(next);
}
```
