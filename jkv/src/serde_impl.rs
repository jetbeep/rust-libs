use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use serde::de::IntoDeserializer;
use serde::ser::SerializeMap;

use crate::{JkvError, JkvKey, JkvValue};

#[derive(Debug, Clone, PartialEq)]
pub struct JkvSerdeError {
    message: String,
}

impl JkvSerdeError {
    pub fn custom(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl fmt::Display for JkvSerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl core::error::Error for JkvSerdeError {}

impl serde::de::Error for JkvSerdeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::custom(msg.to_string())
    }
}

impl serde::ser::Error for JkvSerdeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::custom(msg.to_string())
    }
}

#[derive(Debug, PartialEq)]
pub enum JkvTypedError {
    Codec(JkvError),
    Serde(JkvSerdeError),
}

impl fmt::Display for JkvTypedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JkvTypedError::Codec(e) => write!(f, "{}", e),
            JkvTypedError::Serde(e) => write!(f, "{}", e),
        }
    }
}

impl From<JkvError> for JkvTypedError {
    fn from(value: JkvError) -> Self {
        JkvTypedError::Codec(value)
    }
}

impl From<JkvSerdeError> for JkvTypedError {
    fn from(value: JkvSerdeError) -> Self {
        JkvTypedError::Serde(value)
    }
}

pub fn to_jkv_value<T: serde::Serialize>(value: &T) -> Result<JkvValue, JkvSerdeError> {
    value.serialize(JkvSerializer)
}

pub fn from_jkv_value<T: serde::de::DeserializeOwned>(value: JkvValue) -> Result<T, JkvSerdeError> {
    T::deserialize(ValueDeserializer::new(value))
}

struct JkvSerializer;

impl serde::Serializer for JkvSerializer {
    type Ok = JkvValue;
    type Error = JkvSerdeError;
    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::Int(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        let out = i32::try_from(v).map_err(|_| JkvSerdeError::custom("i64 value out of i32 range"))?;
        Ok(JkvValue::Int(out))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        let out = i32::try_from(v).map_err(|_| JkvSerdeError::custom("u32 value out of i32 range"))?;
        Ok(JkvValue::Int(out))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let out = i32::try_from(v).map_err(|_| JkvSerdeError::custom("u64 value out of i32 range"))?;
        Ok(JkvValue::Int(out))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        if !v.is_finite() {
            return Err(JkvSerdeError::custom("non-finite f32 is not supported"));
        }
        Ok(JkvValue::Float(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        if !v.is_finite() || v.abs() > f32::MAX as f64 {
            return Err(JkvSerdeError::custom("f64 value out of f32 range"));
        }
        Ok(JkvValue::Float(v as f32))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let mut items = Vec::with_capacity(v.len());
        for byte in v {
            items.push(JkvValue::Int(i32::from(*byte)));
        }
        Ok(JkvValue::Array(items))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::Null)
    }

    fn serialize_some<T: ?Sized + serde::Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let val = value.serialize(JkvSerializer)?;
        Ok(JkvValue::Collection(vec![(
            JkvKey::String(variant.to_string()),
            val,
        )]))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer {
            values: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer {
            variant: variant.to_string(),
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            pending_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer {
            variant: variant.to_string(),
            entries: Vec::with_capacity(len),
        })
    }
}

struct SeqSerializer {
    values: Vec<JkvValue>,
}

impl serde::ser::SerializeSeq for SeqSerializer {
    type Ok = JkvValue;
    type Error = JkvSerdeError;

    fn serialize_element<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.values.push(value.serialize(JkvSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::Array(self.values))
    }
}

impl serde::ser::SerializeTuple for SeqSerializer {
    type Ok = JkvValue;
    type Error = JkvSerdeError;

    fn serialize_element<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleStruct for SeqSerializer {
    type Ok = JkvValue;
    type Error = JkvSerdeError;

    fn serialize_field<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

struct TupleVariantSerializer {
    variant: String,
    values: Vec<JkvValue>,
}

impl serde::ser::SerializeTupleVariant for TupleVariantSerializer {
    type Ok = JkvValue;
    type Error = JkvSerdeError;

    fn serialize_field<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.values.push(value.serialize(JkvSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::Collection(vec![(
            JkvKey::String(self.variant),
            JkvValue::Array(self.values),
        )]))
    }
}

struct MapSerializer {
    entries: Vec<(JkvKey, JkvValue)>,
    pending_key: Option<JkvKey>,
}

impl SerializeMap for MapSerializer {
    type Ok = JkvValue;
    type Error = JkvSerdeError;

    fn serialize_key<T: ?Sized + serde::Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        let raw_key = key.serialize(JkvSerializer)?;
        self.pending_key = Some(to_key(raw_key)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self
            .pending_key
            .take()
            .ok_or_else(|| JkvSerdeError::custom("serialize_value called before serialize_key"))?;
        let val = value.serialize(JkvSerializer)?;
        self.entries.push((key, val));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.pending_key.is_some() {
            return Err(JkvSerdeError::custom("map key without value"));
        }
        Ok(JkvValue::Collection(self.entries))
    }
}

impl serde::ser::SerializeStruct for MapSerializer {
    type Ok = JkvValue;
    type Error = JkvSerdeError;

    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let val = value.serialize(JkvSerializer)?;
        self.entries.push((JkvKey::String(key.to_string()), val));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeMap::end(self)
    }
}

struct StructVariantSerializer {
    variant: String,
    entries: Vec<(JkvKey, JkvValue)>,
}

impl serde::ser::SerializeStructVariant for StructVariantSerializer {
    type Ok = JkvValue;
    type Error = JkvSerdeError;

    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let val = value.serialize(JkvSerializer)?;
        self.entries.push((JkvKey::String(key.to_string()), val));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JkvValue::Collection(vec![(
            JkvKey::String(self.variant),
            JkvValue::Collection(self.entries),
        )]))
    }
}

fn to_key(value: JkvValue) -> Result<JkvKey, JkvSerdeError> {
    match value {
        JkvValue::Undefined => Ok(JkvKey::Undefined),
        JkvValue::Null => Ok(JkvKey::Null),
        JkvValue::Bool(v) => Ok(JkvKey::Bool(v)),
        JkvValue::Int(v) => Ok(JkvKey::Int(v)),
        JkvValue::Float(v) => Ok(JkvKey::Float(v)),
        JkvValue::String(v) => Ok(JkvKey::String(v)),
        JkvValue::Collection(_) | JkvValue::Array(_) => {
            Err(JkvSerdeError::custom("collection/map keys must be primitive values"))
        }
    }
}

struct ValueDeserializer {
    value: JkvValue,
}

impl ValueDeserializer {
    fn new(value: JkvValue) -> Self {
        Self { value }
    }
}

impl<'de> serde::Deserializer<'de> for ValueDeserializer {
    type Error = JkvSerdeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Undefined | JkvValue::Null => visitor.visit_unit(),
            JkvValue::Bool(v) => visitor.visit_bool(v),
            JkvValue::Int(v) => visitor.visit_i32(v),
            JkvValue::Float(v) => visitor.visit_f32(v),
            JkvValue::String(v) => visitor.visit_string(v),
            JkvValue::Array(values) => {
                let mut seq = SeqDeserializer {
                    iter: values.into_iter(),
                };
                visitor.visit_seq(&mut seq)
            }
            JkvValue::Collection(entries) => {
                let mut map = MapDeserializer {
                    iter: entries.into_iter(),
                    pending_value: None,
                };
                visitor.visit_map(&mut map)
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Bool(v) => visitor.visit_bool(v),
            other => Err(JkvSerdeError::custom(format!("expected bool, got {other:?}"))),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Int(v) => {
                let out = i8::try_from(v).map_err(|_| JkvSerdeError::custom("i32 out of i8 range"))?;
                visitor.visit_i8(out)
            }
            other => Err(JkvSerdeError::custom(format!("expected i8, got {other:?}"))),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Int(v) => {
                let out = i16::try_from(v).map_err(|_| JkvSerdeError::custom("i32 out of i16 range"))?;
                visitor.visit_i16(out)
            }
            other => Err(JkvSerdeError::custom(format!("expected i16, got {other:?}"))),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Int(v) => visitor.visit_i32(v),
            other => Err(JkvSerdeError::custom(format!("expected i32, got {other:?}"))),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Int(v) => visitor.visit_i64(i64::from(v)),
            other => Err(JkvSerdeError::custom(format!("expected i64, got {other:?}"))),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Int(v) => {
                let out = u8::try_from(v).map_err(|_| JkvSerdeError::custom("i32 out of u8 range"))?;
                visitor.visit_u8(out)
            }
            other => Err(JkvSerdeError::custom(format!("expected u8, got {other:?}"))),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Int(v) => {
                let out = u16::try_from(v).map_err(|_| JkvSerdeError::custom("i32 out of u16 range"))?;
                visitor.visit_u16(out)
            }
            other => Err(JkvSerdeError::custom(format!("expected u16, got {other:?}"))),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Int(v) => {
                let out = u32::try_from(v).map_err(|_| JkvSerdeError::custom("i32 out of u32 range"))?;
                visitor.visit_u32(out)
            }
            other => Err(JkvSerdeError::custom(format!("expected u32, got {other:?}"))),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Int(v) => {
                let out = u64::try_from(v).map_err(|_| JkvSerdeError::custom("i32 out of u64 range"))?;
                visitor.visit_u64(out)
            }
            other => Err(JkvSerdeError::custom(format!("expected u64, got {other:?}"))),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Float(v) => visitor.visit_f32(v),
            JkvValue::Int(v) => visitor.visit_f32(v as f32),
            other => Err(JkvSerdeError::custom(format!("expected f32, got {other:?}"))),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Float(v) => visitor.visit_f64(v as f64),
            JkvValue::Int(v) => visitor.visit_f64(v as f64),
            other => Err(JkvSerdeError::custom(format!("expected f64, got {other:?}"))),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::String(v) => {
                let mut chars = v.chars();
                if let (Some(ch), None) = (chars.next(), chars.next()) {
                    visitor.visit_char(ch)
                } else {
                    Err(JkvSerdeError::custom("expected single-character string"))
                }
            }
            other => Err(JkvSerdeError::custom(format!("expected char, got {other:?}"))),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::String(v) => visitor.visit_string(v),
            other => Err(JkvSerdeError::custom(format!("expected string, got {other:?}"))),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Array(values) => {
                let mut out = Vec::with_capacity(values.len());
                for value in values {
                    match value {
                        JkvValue::Int(v) => {
                            let byte = u8::try_from(v)
                                .map_err(|_| JkvSerdeError::custom("byte array item out of u8 range"))?;
                            out.push(byte);
                        }
                        other => {
                            return Err(JkvSerdeError::custom(format!(
                                "expected byte array item int, got {other:?}"
                            )));
                        }
                    }
                }
                visitor.visit_byte_buf(out)
            }
            other => Err(JkvSerdeError::custom(format!("expected bytes, got {other:?}"))),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Undefined | JkvValue::Null => visitor.visit_none(),
            other => visitor.visit_some(ValueDeserializer::new(other)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Undefined | JkvValue::Null => visitor.visit_unit(),
            other => Err(JkvSerdeError::custom(format!("expected unit, got {other:?}"))),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Array(values) => {
                let mut seq = SeqDeserializer {
                    iter: values.into_iter(),
                };
                visitor.visit_seq(&mut seq)
            }
            other => Err(JkvSerdeError::custom(format!("expected seq, got {other:?}"))),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::Collection(entries) => {
                let mut map = MapDeserializer {
                    iter: entries.into_iter(),
                    pending_value: None,
                };
                visitor.visit_map(&mut map)
            }
            other => Err(JkvSerdeError::custom(format!("expected map, got {other:?}"))),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            JkvValue::String(variant) => visitor.visit_enum(variant.into_deserializer()),
            JkvValue::Collection(mut entries) => {
                if entries.len() != 1 {
                    return Err(JkvSerdeError::custom("expected single-key enum map"));
                }
                let (key, value) = entries.remove(0);
                visitor.visit_enum(EnumDeserializer { key, value })
            }
            other => Err(JkvSerdeError::custom(format!("expected enum, got {other:?}"))),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct SeqDeserializer {
    iter: alloc::vec::IntoIter<JkvValue>,
}

impl<'de> serde::de::SeqAccess<'de> for SeqDeserializer {
    type Error = JkvSerdeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(ValueDeserializer::new(value)).map(Some),
            None => Ok(None),
        }
    }
}

struct MapDeserializer {
    iter: alloc::vec::IntoIter<(JkvKey, JkvValue)>,
    pending_value: Option<JkvValue>,
}

impl<'de> serde::de::MapAccess<'de> for MapDeserializer {
    type Error = JkvSerdeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if let Some((key, value)) = self.iter.next() {
            self.pending_value = Some(value);
            let key_value = key_to_value(key);
            seed.deserialize(ValueDeserializer::new(key_value)).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let value = self
            .pending_value
            .take()
            .ok_or_else(|| JkvSerdeError::custom("map value requested before key"))?;
        seed.deserialize(ValueDeserializer::new(value))
    }
}

fn key_to_value(key: JkvKey) -> JkvValue {
    match key {
        JkvKey::Undefined => JkvValue::Undefined,
        JkvKey::Null => JkvValue::Null,
        JkvKey::Bool(v) => JkvValue::Bool(v),
        JkvKey::Int(v) => JkvValue::Int(v),
        JkvKey::Float(v) => JkvValue::Float(v),
        JkvKey::String(v) => JkvValue::String(v),
    }
}

struct EnumDeserializer {
    key: JkvKey,
    value: JkvValue,
}

impl<'de> serde::de::EnumAccess<'de> for EnumDeserializer {
    type Error = JkvSerdeError;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let key_value = key_to_value(self.key);
        let variant = seed.deserialize(ValueDeserializer::new(key_value))?;
        Ok((variant, VariantDeserializer { value: self.value }))
    }
}

struct VariantDeserializer {
    value: JkvValue,
}

impl<'de> serde::de::VariantAccess<'de> for VariantDeserializer {
    type Error = JkvSerdeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            JkvValue::Null | JkvValue::Undefined => Ok(()),
            other => Err(JkvSerdeError::custom(format!(
                "expected unit variant payload, got {other:?}"
            ))),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(ValueDeserializer::new(self.value))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_seq(ValueDeserializer::new(self.value), visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_map(ValueDeserializer::new(self.value), visitor)
    }
}
