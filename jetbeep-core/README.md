# jetbeep-core

`jetbeep-core` provides the shared runtime APIs used by Jetbeep screen
applications. It keeps application code consistent across the desktop SDL
runtime and Zephyr firmware while each platform supplies its own filesystem,
bus, executor, work queue, logging, time, and LVGL integration.

## Platform features

Exactly one platform feature must be enabled.

| Feature | Default | Environment | Notes |
| --- | --- | --- | --- |
| `platform-desktop` | Yes | Host with `std` | Host filesystem, HTTP agent, desktop executor and LVGL bindings |
| `platform-zephyr` | No | Zephyr build | `no_std`, generated C bindings, Zephyr filesystem and work queue |
| `simulator` | No | Desktop only | Enables locker simulator state and UI; also enables `platform-desktop` |

The desktop and Zephyr features are mutually exclusive. A build with neither
feature is also rejected.

```toml
# Desktop application
[dependencies]
jetbeep-core = { path = "../rust-libs/jetbeep-core" }

# Zephyr application
[dependencies]
jetbeep-core = {
    path = "../rust-libs/jetbeep-core",
    default-features = false,
    features = ["platform-zephyr"],
}
```

## Initialization

Desktop applications may mount their virtual filesystem below a host path:

```rust
jetbeep_core::init(Some("./fs"));
log::info!("runtime initialized at {} us", jetbeep_core::unix_time());
```

On Zephyr, call `jetbeep_core::init(None)` from Rust or invoke the exported
`jetbeep_rust_init` entry point from C. Platform startup must initialize LVGL
and the underlying C services before APIs that depend on them are used.

## Modules

| Module | Purpose | Availability |
| --- | --- | --- |
| `app_launcher` | Deferred soft-kill and application launch requests | Desktop and Zephyr |
| `bus` | Async locker, modem, battery, version, keypad, and server-request APIs | Desktop and Zephyr |
| `error` | Shared platform error type and Zephyr error conversion | Desktop and Zephyr |
| `executor` | Run application futures and cancel stale generations | Desktop and Zephyr |
| `fs` | Common async file and directory interface | Desktop and Zephyr |
| `generation` | Detect callbacks and tasks left over from a previous app generation | Desktop and Zephyr |
| `jkv_file` | Read and write typed or raw header-framed JKV files | Desktop and Zephyr |
| `lvgl` | Platform LVGL surface | Desktop and Zephyr |
| `proto` | Prost types generated from the public protocol subset in `proto/` | Desktop and Zephyr |
| `workq` | Schedule, restart, and cancel delayed work | Desktop and Zephyr |
| `c_bindings` | Bindgen output for platform C APIs | Zephyr |
| `lvgl_fs_driver` | Register desktop LVGL drive `J:` below the mounted filesystem | Desktop |
| `simulator` | Locker layout catalog, state, configuration editor, and simulator UI | `simulator` feature |

The crate root also exposes `init`, `unix_time`, desktop `init_agent`, and
simulator `init_simulator` entry points when their features are active.

## Generated interfaces

The build script always compiles the canonical public protobuf subset under
`proto/` with vendored `protoc`. Zephyr builds additionally generate C bindings
from `bindgen/wrapper.h`; the enclosing Zephyr/CMake build supplies
`ZEPHYR_BASE`, target and generated include paths, include definitions, and
`autoconf.h`.

Host builds therefore need a working Rust C toolchain and libclang for
`bindgen`. Zephyr builds should be driven by the parent firmware build rather
than invoking the crate independently.

## Development

Run the desktop tests from the workspace root:

```sh
cargo test -p jetbeep-core
```

Use the parent project's normal Zephyr build to validate `platform-zephyr`.
