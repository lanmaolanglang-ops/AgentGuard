use crate::{ProtocolError, Result};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

pub const MAX_FRAME_BYTES: usize = 1_048_576;
pub const MAX_DEPTH: usize = 8;
pub const MAX_INTEGER: i64 = 9_007_199_254_740_991;

struct ValueSeed {
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for ValueSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        if self.depth > MAX_DEPTH {
            return Err(de::Error::custom("maximum nesting depth"));
        }
        deserializer.deserialize_any(StrictVisitor { depth: self.depth })
    }
}

struct StrictVisitor {
    depth: usize,
}

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bounded JSON with unique keys and safe integers")
    }

    fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> std::result::Result<Value, E> {
        if !(-MAX_INTEGER..=MAX_INTEGER).contains(&value) {
            return Err(E::custom("unsafe integer"));
        }
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Value, E> {
        if value > MAX_INTEGER as u64 {
            return Err(E::custom("unsafe integer"));
        }
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E: de::Error>(self, _value: f64) -> std::result::Result<Value, E> {
        Err(E::custom(
            "floating point, exponent and negative zero are forbidden",
        ))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> std::result::Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> std::result::Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(ValueSeed {
            depth: self.depth + 1,
        })? {
            if values.len() == 128 {
                return Err(de::Error::custom("array budget"));
            }
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> std::result::Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate key"));
            }
            if values.len() == 64 {
                return Err(de::Error::custom("map budget"));
            }
            let value = access.next_value_seed(ValueSeed {
                depth: self.depth + 1,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

/// Parse before schema validation, preserving lexical integer and duplicate-key rules.
pub fn parse_json(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::new(
            "frame_limit",
            "请求过大，本次请求已拒绝。",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = ValueSeed { depth: 0 }
        .deserialize(&mut deserializer)
        .map_err(|_| {
            ProtocolError::new(
                "invalid_json",
                "请求格式、数字或嵌套结构不符合协议，本次请求已拒绝。",
            )
        })?;
    deserializer
        .end()
        .map_err(|_| ProtocolError::new("trailing_data", "请求包含额外数据，本次请求已拒绝。"))?;
    Ok(value)
}
