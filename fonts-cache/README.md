# fonts-cache

`fonts-cache` is a `no_std`, on-demand cache for LVGL binary fonts. It gives
LVGL a stable proxy `lv_font_t` pointer for each font while loading and evicting
the underlying glyph data under a configurable least-recently-used byte
budget.

The crate is intended for a single LVGL thread. It uses `alloc` and references
LVGL's `lv_binfont_*` and `lv_fs_*` C APIs; the final application must provide
those symbols.

## Add the crate

```toml
[dependencies]
fonts-cache = { path = "../rust-libs/fonts-cache" }
lvgl-dsl = { path = "../rust-libs/lvgl-dsl" }
```

## Usage

Initialize the cache with an LVGL filesystem prefix and resident byte budget,
then request fonts by filename without the `.bin` suffix:

```rust
use lvgl_dsl::Font;

let fallback = Font::montserrat_20();
fonts_cache::init("J:fonts/", 256 * 1024);

let proxy = fonts_cache::get("Poppins-Regular-20", fallback.as_ptr().cast());

// Safe because cache proxy addresses remain valid for the process lifetime.
let font = unsafe { Font::from_raw(proxy.cast()) };
```

The example resolves `J:fonts/Poppins-Regular-20.bin`. The fallback must be a
valid, long-lived LVGL font pointer. A null fallback is accepted, but rendering
cannot recover when the binary font fails to load.

Call `warm` on the LVGL thread to pay the parsing and glyph-table setup cost
before first rendering:

```rust
let loaded_real_font = fonts_cache::warm(
    "Poppins-Regular-20",
    fallback.as_ptr().cast(),
);
```

## API

| Function | Purpose |
| --- | --- |
| `init(base_dir, budget)` | Set the LVGL directory prefix and resident byte budget |
| `get(name, fallback)` | Return the permanent proxy for `<base_dir><name>.bin` |
| `warm(name, fallback)` | Create the proxy and load the real font immediately |
| `set_budget(bytes)` | Change the budget and evict entries if necessary |
| `loaded_bytes()` | Report the estimated bytes currently resident |

Calling `get` repeatedly with the same name returns the same proxy. Eviction
destroys only the real loaded font, never the proxy held by widgets or styles;
the next glyph access reloads it. Failed loads bind the proxy to its fallback
and do not count against the resident budget.

## Integration constraints

- Call every API from the LVGL thread. The global cache is intentionally not a
  general-purpose thread-safe container.
- Configure an LVGL filesystem driver for the `base_dir` drive letter before a
  font is loaded.
- Supply LVGL binary font files compatible with the linked LVGL version.
- The crate mirrors the public `lv_font_t` layout and checks its size for
  32-bit and 64-bit targets.

## Development

From the workspace root:

```sh
cargo check -p fonts-cache
```
