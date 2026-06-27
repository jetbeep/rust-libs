#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ops::Index;

mod serde_impl;

pub use serde_impl::{from_jkv_value, to_jkv_value, JkvSerdeError, JkvTypedError};

pub const JKV_TAG: &[u8; 3] = b"JKV";
pub const JKV_VERSION: u8 = 1;
pub const JKV_HEADER_SIZE: usize = 4;
pub const MAX_VALIDATION_DEPTH: usize = 250;

const TYPE_UNDEFINED: u8 = 0;
const TYPE_NULL: u8 = 1;
const TYPE_BOOL: u8 = 2;
const TYPE_POSITIVE_INT32: u8 = 3;
const TYPE_NEGATIVE_INT32: u8 = 4;
const TYPE_FLOAT: u8 = 5;
const TYPE_STRING: u8 = 6;
const TYPE_COLLECTION: u8 = 7;
const TYPE_ARRAY: u8 = 8;

#[derive(Debug, Clone, PartialEq)]
pub enum JkvKey {
    Int(i32),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum JkvValue {
    #[default]
    Undefined,
    Null,
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Collection(Vec<(JkvKey, JkvValue)>),
    Array(Vec<JkvValue>),
}

static JKV_UNDEFINED_VALUE: JkvValue = JkvValue::Undefined;

impl JkvValue {
    pub fn get_key(&self, key: &str) -> Option<&JkvValue> {
        let JkvValue::Collection(entries) = self else {
            return None;
        };

        // Keep duplicate keys representable, but access behaves like JS objects:
        // the last assignment wins.
        entries.iter().rev().find_map(|(k, v)| {
            if matches!(k, JkvKey::String(s) if s == key) {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn get_index(&self, index: usize) -> Option<&JkvValue> {
        match self {
            JkvValue::Array(items) => items.get(index),
            JkvValue::Collection(entries) => {
                let key = i32::try_from(index).ok()?;
                entries.iter().rev().find_map(|(k, v)| {
                    if matches!(k, JkvKey::Int(i) if *i == key) {
                        Some(v)
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }
}

impl Index<&str> for JkvValue {
    type Output = JkvValue;

    fn index(&self, key: &str) -> &Self::Output {
        self.get_key(key).unwrap_or(&JKV_UNDEFINED_VALUE)
    }
}

impl Index<usize> for JkvValue {
    type Output = JkvValue;

    fn index(&self, index: usize) -> &Self::Output {
        self.get_index(index).unwrap_or(&JKV_UNDEFINED_VALUE)
    }
}

#[derive(Debug, PartialEq)]
pub enum JkvError {
    UnexpectedEof,
    InvalidTag,
    UnsupportedVersion(u8),
    InvalidType(u8),
    InvalidIntWidth(u8),
    MalformedString,
    StringContainsNull,
    IntOutOfRange,
    IntMinNotSupported,
    NonPrimitiveCollectionKey,
    TrailingBytes,
    MaxDepthExceeded,
}

impl core::fmt::Display for JkvError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            JkvError::UnexpectedEof => write!(f, "data too short"),
            JkvError::InvalidTag => write!(f, "invalid header tag"),
            JkvError::UnsupportedVersion(v) => write!(f, "unsupported version: {}", v),
            JkvError::InvalidType(v) => write!(f, "invalid type: {}", v),
            JkvError::InvalidIntWidth(v) => write!(f, "invalid integer width: {}", v),
            JkvError::MalformedString => write!(f, "malformed string (missing null terminator)"),
            JkvError::StringContainsNull => write!(f, "string contains interior null"),
            JkvError::IntOutOfRange => write!(f, "integer out of range for i32"),
            JkvError::IntMinNotSupported => write!(f, "cannot encode i32::MIN as negative int32"),
            JkvError::NonPrimitiveCollectionKey => write!(f, "collection key must be primitive"),
            JkvError::TrailingBytes => write!(f, "trailing bytes after top-level value"),
            JkvError::MaxDepthExceeded => write!(f, "nesting too deep"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JkvWriteStream {
    data: Vec<u8>,
    min_alloc_size: usize,
}

impl JkvWriteStream {
    pub fn new(min_alloc_size: usize) -> Self {
        Self {
            data: Vec::new(),
            min_alloc_size,
        }
    }

    pub fn encoded_size(&self) -> usize {
        self.data.len()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    pub fn write_header(&mut self) {
        self.ensure_bytes(JKV_HEADER_SIZE);
        self.data.extend_from_slice(JKV_TAG);
        self.data.push(JKV_VERSION);
    }

    pub fn write_data(&mut self, bytes: &[u8]) {
        self.ensure_bytes(bytes.len());
        self.data.extend_from_slice(bytes);
    }

    pub fn write_value(&mut self, value: &JkvValue) -> Result<(), JkvError> {
        self.ensure_bytes(8);
        encode_value_into(value, &mut self.data, 0)
    }

    fn ensure_bytes(&mut self, bytes: usize) {
        let needed = self.data.len().saturating_add(bytes);
        if needed > self.data.capacity() {
            let grow_by = self.min_alloc_size.max(bytes);
            self.data.reserve(grow_by);
        }
    }
}

pub fn decoder_init(data: &[u8]) -> Result<usize, JkvError> {
    if data.len() < JKV_HEADER_SIZE {
        return Err(JkvError::UnexpectedEof);
    }
    if &data[..3] != JKV_TAG {
        return Err(JkvError::InvalidTag);
    }
    if data[3] != JKV_VERSION {
        return Err(JkvError::UnsupportedVersion(data[3]));
    }
    Ok(JKV_HEADER_SIZE)
}

pub fn encode(value: &JkvValue) -> Result<Vec<u8>, JkvError> {
    let mut out = Vec::new();
    encode_value_into(value, &mut out, 0)?;
    Ok(out)
}

pub fn encode_with_header(value: &JkvValue) -> Result<Vec<u8>, JkvError> {
    let mut stream = JkvWriteStream::new(64);
    stream.write_header();
    stream.write_value(value)?;
    Ok(stream.into_inner())
}

pub fn decode(data: &[u8]) -> Result<JkvValue, JkvError> {
    if data.is_empty() {
        return Err(JkvError::UnexpectedEof);
    }
    let mut offset = 0;
    let value = decode_value_at(data, &mut offset, 0)?;
    if offset != data.len() {
        return Err(JkvError::TrailingBytes);
    }
    Ok(value)
}

pub fn decode_with_header(data: &[u8]) -> Result<JkvValue, JkvError> {
    let mut offset = decoder_init(data)?;
    if offset >= data.len() {
        return Err(JkvError::UnexpectedEof);
    }
    let value = decode_value_at(data, &mut offset, 0)?;
    if offset != data.len() {
        return Err(JkvError::TrailingBytes);
    }
    Ok(value)
}

pub fn validate(data: &[u8], with_header: bool) -> Result<(), JkvError> {
    let mut start = 0;
    if with_header {
        start = decoder_init(data)?;
    }
    if start == data.len() {
        return Ok(());
    }
    let mut offset = start;
    let _ = decode_value_at(data, &mut offset, 0)?;
    if offset != data.len() {
        return Err(JkvError::TrailingBytes);
    }
    Ok(())
}

pub fn from_slice<T>(data: &[u8]) -> Result<T, JkvTypedError>
where
    T: serde::de::DeserializeOwned,
{
    let value = decode(data)?;
    from_jkv_value(value).map_err(JkvTypedError::from)
}

pub fn from_slice_with_header<T>(data: &[u8]) -> Result<T, JkvTypedError>
where
    T: serde::de::DeserializeOwned,
{
    let value = decode_with_header(data)?;
    from_jkv_value(value).map_err(JkvTypedError::from)
}

pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, JkvTypedError>
where
    T: serde::Serialize,
{
    let jkv = to_jkv_value(value).map_err(JkvTypedError::from)?;
    encode(&jkv).map_err(JkvTypedError::from)
}

pub fn to_vec_with_header<T>(value: &T) -> Result<Vec<u8>, JkvTypedError>
where
    T: serde::Serialize,
{
    let jkv = to_jkv_value(value).map_err(JkvTypedError::from)?;
    encode_with_header(&jkv).map_err(JkvTypedError::from)
}

fn encode_value_into(value: &JkvValue, out: &mut Vec<u8>, depth: usize) -> Result<(), JkvError> {
    if depth + 1 >= MAX_VALIDATION_DEPTH {
        return Err(JkvError::MaxDepthExceeded);
    }

    match value {
        JkvValue::Undefined => encode_tag_only(TYPE_UNDEFINED, out),
        JkvValue::Null => encode_tag_only(TYPE_NULL, out),
        JkvValue::Bool(v) => {
            out.push((TYPE_BOOL << 4) | u8::from(*v));
        }
        JkvValue::Int(v) => encode_i32(*v, out)?,
        JkvValue::Float(v) => encode_u32_payload(TYPE_FLOAT, v.to_bits(), out),
        JkvValue::String(v) => {
            if v.as_bytes().contains(&0) {
                return Err(JkvError::StringContainsNull);
            }
            out.push(TYPE_STRING << 4);
            out.extend_from_slice(v.as_bytes());
            out.push(0);
        }
        JkvValue::Collection(entries) => {
            let count = u32::try_from(entries.len()).map_err(|_| JkvError::IntOutOfRange)?;
            encode_u32_payload(TYPE_COLLECTION, count, out);
            for (k, v) in entries {
                encode_key_into(k, out)?;
                encode_value_into(v, out, depth + 1)?;
            }
        }
        JkvValue::Array(items) => {
            let count = u32::try_from(items.len()).map_err(|_| JkvError::IntOutOfRange)?;
            encode_u32_payload(TYPE_ARRAY, count, out);
            for item in items {
                encode_value_into(item, out, depth + 1)?;
            }
        }
    }
    Ok(())
}

fn encode_key_into(key: &JkvKey, out: &mut Vec<u8>) -> Result<(), JkvError> {
    match key {
        JkvKey::Int(v) => encode_i32(*v, out)?,
        JkvKey::String(v) => {
            if v.as_bytes().contains(&0) {
                return Err(JkvError::StringContainsNull);
            }
            out.push(TYPE_STRING << 4);
            out.extend_from_slice(v.as_bytes());
            out.push(0);
        }
    }
    Ok(())
}

fn encode_i32(value: i32, out: &mut Vec<u8>) -> Result<(), JkvError> {
    if value >= 0 {
        encode_u32_payload(TYPE_POSITIVE_INT32, value as u32, out);
        return Ok(());
    }

    if value == i32::MIN {
        return Err(JkvError::IntMinNotSupported);
    }

    let abs = (-value) as u32;
    encode_u32_payload(TYPE_NEGATIVE_INT32, abs, out);
    Ok(())
}

fn encode_u32_payload(tag: u8, payload: u32, out: &mut Vec<u8>) {
    let (buf, n) = encode_uint_le_trimmed(payload);
    out.push((tag << 4) | (n as u8));
    out.extend_from_slice(&buf[..n]);
}

fn encode_tag_only(tag: u8, out: &mut Vec<u8>) {
    out.push(tag << 4);
}

fn encode_uint_le_trimmed(value: u32) -> ([u8; 4], usize) {
    let bytes = value.to_le_bytes();
    let mut n = 4usize;
    while n > 0 && bytes[n - 1] == 0 {
        n -= 1;
    }
    (bytes, n)
}

fn decode_value_at(data: &[u8], offset: &mut usize, depth: usize) -> Result<JkvValue, JkvError> {
    if depth + 1 >= MAX_VALIDATION_DEPTH {
        return Err(JkvError::MaxDepthExceeded);
    }
    if *offset >= data.len() {
        return Err(JkvError::UnexpectedEof);
    }

    let b0 = data[*offset];
    *offset += 1;

    let tag = b0 >> 4;
    let low = b0 & 0x0F;

    match tag {
        TYPE_UNDEFINED => Ok(JkvValue::Undefined),
        TYPE_NULL => Ok(JkvValue::Null),
        TYPE_BOOL => Ok(JkvValue::Bool((low & 0x01) != 0)),
        TYPE_POSITIVE_INT32 => {
            let raw = decode_u32_payload(data, offset, low)?;
            let val = i32::try_from(raw).map_err(|_| JkvError::IntOutOfRange)?;
            Ok(JkvValue::Int(val))
        }
        TYPE_NEGATIVE_INT32 => {
            let raw = decode_u32_payload(data, offset, low)?;
            let pos = i32::try_from(raw).map_err(|_| JkvError::IntOutOfRange)?;
            if pos == i32::MIN {
                return Err(JkvError::IntOutOfRange);
            }
            Ok(JkvValue::Int(-pos))
        }
        TYPE_FLOAT => {
            let bits = decode_u32_payload(data, offset, low)?;
            Ok(JkvValue::Float(f32::from_bits(bits)))
        }
        TYPE_STRING => {
            let start = *offset;
            let rest = &data[start..];
            let nul_pos = rest.iter().position(|b| *b == 0).ok_or(JkvError::MalformedString)?;
            let s = core::str::from_utf8(&rest[..nul_pos]).map_err(|_| JkvError::MalformedString)?;
            *offset = start + nul_pos + 1;
            Ok(JkvValue::String(s.to_string()))
        }
        TYPE_COLLECTION => {
            let count = decode_u32_payload(data, offset, low)? as usize;
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                let key = decode_key_at(data, offset)?;
                let val = decode_value_at(data, offset, depth + 1)?;
                entries.push((key, val));
            }
            Ok(JkvValue::Collection(entries))
        }
        TYPE_ARRAY => {
            let count = decode_u32_payload(data, offset, low)? as usize;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push(decode_value_at(data, offset, depth + 1)?);
            }
            Ok(JkvValue::Array(values))
        }
        _ => Err(JkvError::InvalidType(tag)),
    }
}

fn decode_key_at(data: &[u8], offset: &mut usize) -> Result<JkvKey, JkvError> {
    let key_value = decode_value_at(data, offset, 0)?;
    match key_value {
        JkvValue::Int(v) => Ok(JkvKey::Int(v)),
        JkvValue::String(v) => Ok(JkvKey::String(v)),
        _ => Err(JkvError::NonPrimitiveCollectionKey),
    }
}

fn decode_u32_payload(data: &[u8], offset: &mut usize, width: u8) -> Result<u32, JkvError> {
    if width > 4 {
        return Err(JkvError::InvalidIntWidth(width));
    }
    let width_usize = width as usize;
    if data.len().saturating_sub(*offset) < width_usize {
        return Err(JkvError::UnexpectedEof);
    }
    let mut buf = [0u8; 4];
    buf[..width_usize].copy_from_slice(&data[*offset..*offset + width_usize]);
    *offset += width_usize;
    Ok(u32::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ReportProblemResponseFixture {
        report_id: String,
        feedback_url: String,
    }

    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(untagged)]
    enum IntOrFloatFixture {
        Int(i64),
        Float(f64),
    }

    fn de_i32_lossy_fixture<'de, D>(d: D) -> Result<i32, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        match IntOrFloatFixture::deserialize(d)? {
            IntOrFloatFixture::Int(i) => Ok(i as i32),
            IntOrFloatFixture::Float(f) => Ok(f.round() as i32),
        }
    }

    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct NumericFixture {
        #[serde(deserialize_with = "de_i32_lossy_fixture")]
        image_x: i32,
    }

    #[test]
    fn header_roundtrip_null() {
        let data = encode_with_header(&JkvValue::Null).expect("encode should work");
        assert_eq!(data, vec![b'J', b'K', b'V', 1, 0x10]);

        let decoded = decode_with_header(&data).expect("decode should work");
        assert_eq!(decoded, JkvValue::Null);
    }

    #[test]
    fn primitive_encodings_match_expected_bytes() {
        assert_eq!(encode(&JkvValue::Bool(false)).unwrap(), vec![0x20]);
        assert_eq!(encode(&JkvValue::Bool(true)).unwrap(), vec![0x21]);
        assert_eq!(encode(&JkvValue::Int(0)).unwrap(), vec![0x30]);
        assert_eq!(encode(&JkvValue::Int(-1)).unwrap(), vec![0x41, 0x01]);
        assert_eq!(encode(&JkvValue::String("A".to_string())).unwrap(), vec![0x60, b'A', 0x00]);
    }

    #[test]
    fn collection_preserves_order_and_duplicate_keys() {
        let value = JkvValue::Collection(vec![
            (JkvKey::String("k".into()), JkvValue::Int(1)),
            (JkvKey::String("k".into()), JkvValue::Int(2)),
            (JkvKey::Int(5), JkvValue::Bool(true)),
        ]);

        let encoded = encode_with_header(&value).unwrap();
        let decoded = decode_with_header(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn validation_rejects_non_primitive_collection_key() {
        // collection(1), key = array(0), value = int(1)
        let data = vec![0x71, 0x01, 0x80, 0x31, 0x01];
        let err = validate(&data, false).unwrap_err();
        assert_eq!(err, JkvError::NonPrimitiveCollectionKey);
    }

    #[test]
    fn stream_writer_matches_compose_behavior() {
        let mut stream = JkvWriteStream::new(16);
        stream.write_header();
        stream
            .write_value(&JkvValue::Array(vec![JkvValue::Bool(true), JkvValue::Int(55)]))
            .unwrap();

        assert_eq!(stream.as_slice(), &[b'J', b'K', b'V', 1, 0x81, 0x02, 0x21, 0x31, 55]);
    }

    #[test]
    fn js_style_indexing_reads_nested_values() {
        let value = JkvValue::Collection(vec![
            (
                JkvKey::String("key1".into()),
                JkvValue::Array(vec![
                    JkvValue::Null,
                    JkvValue::Bool(true),
                    JkvValue::Collection(vec![(
                        JkvKey::String("key3".into()),
                        JkvValue::Int(42),
                    )]),
                ]),
            ),
        ]);

        assert_eq!(value["key1"][2]["key3"], JkvValue::Int(42));
    }

    #[test]
    fn js_style_indexing_returns_undefined_for_missing_path() {
        let value = JkvValue::Collection(vec![(
            JkvKey::String("arr".into()),
            JkvValue::Array(vec![JkvValue::Int(1)]),
        )]);

        assert_eq!(value["missing"], JkvValue::Undefined);
        assert_eq!(value["arr"][99], JkvValue::Undefined);
        assert_eq!(value["arr"][0]["nested"], JkvValue::Undefined);
    }

    #[test]
    fn serde_from_slice_supports_camel_case_fields() {
        let jkv_value = JkvValue::Collection(vec![
            (JkvKey::String("reportId".into()), JkvValue::String("RID-1".into())),
            (
                JkvKey::String("feedbackUrl".into()),
                JkvValue::String("https://example.test/fb".into()),
            ),
        ]);
        let payload = encode(&jkv_value).expect("encode should work");

        let parsed: ReportProblemResponseFixture =
            from_slice(&payload).expect("typed parse should succeed");

        assert_eq!(
            parsed,
            ReportProblemResponseFixture {
                report_id: "RID-1".into(),
                feedback_url: "https://example.test/fb".into(),
            }
        );
    }

    #[test]
    fn serde_from_slice_supports_untagged_numeric_deserializer() {
        let jkv_value = JkvValue::Collection(vec![(
            JkvKey::String("imageX".into()),
            JkvValue::Float(12.6),
        )]);
        let payload = encode(&jkv_value).expect("encode should work");

        let parsed: NumericFixture = from_slice(&payload).expect("typed parse should succeed");
        assert_eq!(parsed.image_x, 13);
    }

    #[test]
    fn serde_roundtrip_with_header_works() {
        let value = ReportProblemResponseFixture {
            report_id: "RID-88".into(),
            feedback_url: "https://example.test/ok".into(),
        };

        let bytes = to_vec_with_header(&value).expect("serialize should work");
        let parsed: ReportProblemResponseFixture =
            from_slice_with_header(&bytes).expect("deserialize should work");

        assert_eq!(parsed, value);
    }

    #[test]
    fn js_style_key_lookup_uses_last_duplicate_entry() {
        let value = JkvValue::Collection(vec![
            (JkvKey::String("dup".into()), JkvValue::Int(1)),
            (JkvKey::String("dup".into()), JkvValue::Int(2)),
        ]);

        assert_eq!(value["dup"], JkvValue::Int(2));
    }

    #[test]
    fn usize_indexing_reads_collection_integer_keys() {
        let value = JkvValue::Collection(vec![
            (JkvKey::Int(1), JkvValue::String("one".into())),
            (JkvKey::Int(2), JkvValue::String("two".into())),
        ]);

        assert_eq!(value[1], JkvValue::String("one".into()));
        assert_eq!(value[2], JkvValue::String("two".into()));
    }

    #[test]
    fn usize_indexing_collection_uses_last_duplicate_integer_key() {
        let value = JkvValue::Collection(vec![
            (JkvKey::Int(1), JkvValue::Int(10)),
            (JkvKey::Int(1), JkvValue::Int(20)),
        ]);

        assert_eq!(value[1], JkvValue::Int(20));
    }
}
