# lvgl-dsl

`lvgl-dsl` is a Rust UI layer for LVGL 9. It provides typed widgets, chainable
layout and style methods, closure-based events and animations, static styles,
keyboard layouts, and higher-level controls used by Jetbeep screen
applications.

For the complete widget and method catalog, see
[DSL_REFERENCE.md](DSL_REFERENCE.md). [DSL_PLAYGROUND.html](DSL_PLAYGROUND.html)
is an offline interactive playground for composing examples.

## Add the crate

```toml
[dependencies]
lvgl-dsl = { path = "../rust-libs/lvgl-dsl" }
```

## Usage

The prelude is re-exported at the crate root:

```rust
use lvgl_dsl::prelude::*;

let screen = Screen::new();

let content = Obj::new(&screen)
    .size(Size::Pct(100), Size::Pct(100))
    .flex_col()
    .flex_align(FlexAlign::Center, FlexAlign::Center, FlexAlign::Start)
    .gap(12);

let button = Button::new(&content)
    .size(Size::Px(240), Size::Px(56))
    .bg_color(Color::hex(0x18794E))
    .radius(CornerRadius::Px(6))
    .on_click(|_| { /* handle confirmation */ });

button.text("Confirm");
screen.load();
```

LVGL must already be initialized before widgets are created. Widget handles
refer to C objects owned by LVGL; deleting a parent invalidates handles to its
children.

## API areas

| Area | Main types |
| --- | --- |
| Containers and navigation | `Screen`, `Obj`, `Widget`, `ScreenAnim` |
| Basic controls | `Button`, `Label`, `Image`, `ImageButton`, `Dropdown`, `TextArea`, `Arc`, `Spinner`, `QrCode` |
| Input | `Keyboard`, `KeyboardLayout`, `PhoneFormatterField`, `ButtonMatrix`, `RadioButtonList` |
| Domain controls | `ParcelLocker`, `SearchBar` |
| Layout and appearance | `Size`, `LvAlign`, `FlexFlow`, `FlexAlign`, `Color`, `Palette`, `CornerRadius`, `Style` |
| Compile-time styling | `StaticStyle`, `StaticStyleProp`, static-style macros |
| Resources | `Font`, `ImageSrc`, keyboard key maps and themes |
| Interaction | `Event`, `LvEventCode`, `Anim`, `AnimHandle`, `AnimPath` |
| Low-level interop | `c_bindings`, re-exported LVGL pointer and animation types |

## Build environments

The build script selects its backend from the surrounding build environment.

| Environment | Behavior |
| --- | --- |
| Plain Cargo build | Enables `std`, uses the mock LVGL layer, and exposes test spy hooks |
| `LVGL_INCLUDE_DIRS` set without `ZEPHYR_BASE` | Links the desktop simulator backend, derives style property IDs from real headers, and runs C ABI assertions |
| `ZEPHYR_BASE` set | Builds `no_std`, generates LVGL bindings, and reads enabled Montserrat fonts from Zephyr `autoconf.h` |

The desktop simulator also accepts `LV_CONF_DIR`. The Zephyr integration
supplies `INCLUDE_DIRS`, `INCLUDE_DEFINES`, and
`BINARY_DIR_INCLUDE_GENERATED` through its CMake build.

Built-in `Font::montserrat_*` constructors are emitted only for sizes enabled
in the target LVGL configuration. Application-owned fonts and images can be
introduced through the documented raw descriptor APIs when their pointers are
valid for the required lifetime.

## Development

From the workspace root, the plain Cargo environment exercises the mock LVGL
backend:

```sh
cargo test -p lvgl-dsl
```

The desktop simulator and firmware builds remain the authoritative ABI checks
for their respective LVGL configurations.
