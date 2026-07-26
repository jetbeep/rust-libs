# rust-libs

Shared Rust crates for Jetbeep screen applications.

## Crates

| Crate | Description |
| --- | --- |
| [fonts-cache](fonts-cache/README.md) | `no_std` on-demand LVGL binary font cache with stable proxies and LRU eviction |
| [jetbeep-core](jetbeep-core/README.md) | Cross-platform runtime APIs for desktop screen applications and Zephyr firmware |
| [jkv](jkv/README.md) | `no_std` JKV binary codec with raw values and Serde support |
| [lvgl-dsl](lvgl-dsl/README.md) | Typed LVGL 9 widget, layout, styling, event, and animation DSL |

`jetbeep-core/proto` contains the canonical public protocol subset used by the
desktop SDK. Firmware-only and private protocol definitions are maintained
separately and are not required to build the desktop platform.

## Development

Run host-side checks from this directory:

```sh
cargo test --workspace
cargo doc --workspace --no-deps
```

Plain Cargo builds use the desktop defaults for `jetbeep-core` and the mock
backend for `lvgl-dsl`. Validate Zephyr and the real desktop LVGL integration
through their parent projects, which provide the required C headers, libraries,
and generated configuration.

All crates are licensed under Apache-2.0.
