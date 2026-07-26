# jkv

`jkv` is a `no_std` codec for JetBeep Key-Value (JKV) binary data. It supports
both a raw value tree and typed serialization through Serde, using `alloc`
without requiring the Rust standard library.

## Add the crate

From another crate in this repository:

```toml
[dependencies]
jkv = { path = "../rust-libs/jkv" }
serde = { version = "1", default-features = false, features = ["alloc", "derive"] }
```

## Typed values

Use the `*_with_header` helpers for files and other standalone JKV payloads.
They prepend and validate the four-byte `JKV` + version header.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Settings {
    locale: String,
    retries: u8,
}

let settings = Settings {
    locale: "en".into(),
    retries: 3,
};

let bytes = jkv::to_vec_with_header(&settings)?;
let decoded: Settings = jkv::from_slice_with_header(&bytes)?;
assert_eq!(decoded, settings);
# Ok::<(), jkv::JkvTypedError>(())
```

`to_vec` and `from_slice` use the same value encoding without the file header.

## Raw values

Use `JkvValue` when the schema is dynamic or values must preserve collection
ordering and duplicate keys.

```rust
use jkv::{JkvKey, JkvValue};

let value = JkvValue::Collection(vec![
    (JkvKey::String("enabled".into()), JkvValue::Bool(true)),
    (JkvKey::String("limit".into()), JkvValue::Int(12)),
]);

let bytes = jkv::encode_with_header(&value)?;
let decoded = jkv::decode_with_header(&bytes)?;

assert_eq!(decoded["limit"], JkvValue::Int(12));
assert_eq!(decoded["missing"], JkvValue::Undefined);
# Ok::<(), jkv::JkvError>(())
```

String and integer indexing return `JkvValue::Undefined` for a missing path.
`get_key` and `get_index` return `Option` when that distinction matters.

## API overview

| API | Purpose |
| --- | --- |
| `to_vec`, `from_slice` | Serialize or deserialize typed values without a header |
| `to_vec_with_header`, `from_slice_with_header` | Serialize or deserialize typed standalone payloads |
| `encode`, `decode` | Encode or decode a raw `JkvValue` without a header |
| `encode_with_header`, `decode_with_header` | Encode or decode a raw standalone payload |
| `validate` | Validate framing and value encoding without keeping the result |
| `JkvWriteStream` | Incrementally compose a header and encoded values |
| `to_jkv_value`, `from_jkv_value` | Convert between Serde types and the raw value tree |

## Format notes

- The current file header is `b"JKV\x01"`.
- Values include undefined, null, booleans, signed 32-bit integers, 32-bit
  floats, null-terminated UTF-8 strings, collections, and arrays.
- Collection keys must be strings or 32-bit integers. Ordering and duplicate
  keys are preserved; lookup returns the last matching key.
- Interior null bytes in strings and `i32::MIN` cannot be encoded.
- Validation rejects trailing bytes and nesting at or beyond
  `MAX_VALIDATION_DEPTH`.

## Development

From the workspace root:

```sh
cargo test -p jkv
```
