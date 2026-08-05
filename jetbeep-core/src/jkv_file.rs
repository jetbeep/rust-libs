//! File-backed JKV (JetBeep Key-Value) read/write helpers.
//!
//! Thin wrapper around [`crate::fs`] and the [`jkv`] codec so application code
//! can persist and load typed structures (or raw [`jkv::JkvValue`] trees) in the
//! compact JKV binary format without re-implementing the open/read/write/sync
//! dance every time.
//!
//! The module is platform-agnostic: it builds on the common async `fs::File`
//! API that exists on both the desktop (`platform-desktop`) and Zephyr
//! (`platform-zephyr`) backends, and on the `no_std` `jkv` codec. This makes it
//! usable from firmware, which has no JSON support.
//!
//! All readers/writers operate on the *with-header* JKV framing
//! (`b"JKV"` + version byte), matching [`jkv::encode_with_header`] /
//! [`jkv::decode_with_header`].

use alloc::format;
use jkv::JkvValue;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::Error;

const EINVAL: i32 = 22;

fn codec_error(op: &str, detail: impl core::fmt::Display) -> Error {
    Error {
        code: -EINVAL,
        message: format!("jkv_file::{}: {}", op, detail),
    }
}

/// Load and deserialize a JKV file at `path` into `T`.
///
/// The file must carry the JKV header. Returns an [`Error`] on any fs failure,
/// malformed JKV framing, or serde type mismatch.
pub async fn read<T>(path: &str) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let bytes = crate::binary_file::read(path).await?;
    jkv::from_slice_with_header::<T>(&bytes).map_err(|e| codec_error("read", e))
}

/// Serialize `value` to JKV (with header) and write it to `path`, replacing any
/// existing contents.
pub async fn write<T>(path: &str, value: &T) -> Result<(), Error>
where
    T: Serialize,
{
    let bytes = jkv::to_vec_with_header(value).map_err(|e| codec_error("write", e))?;
    crate::binary_file::write(path, &bytes).await
}

/// Load a JKV file at `path` into a raw [`JkvValue`] tree (no serde mapping).
pub async fn read_value(path: &str) -> Result<JkvValue, Error> {
    let bytes = crate::binary_file::read(path).await?;
    jkv::decode_with_header(&bytes).map_err(|e| codec_error("read_value", e))
}

/// Encode a raw [`JkvValue`] tree (with header) and write it to `path`.
pub async fn write_value(path: &str, value: &JkvValue) -> Result<(), Error> {
    let bytes = jkv::encode_with_header(value).map_err(|e| codec_error("write_value", e))?;
    crate::binary_file::write(path, &bytes).await
}
