use super::Error;
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::cell::Cell;
use std::fmt;

const MAX_JSON_ENTRIES: usize = 16_384;

#[derive(Clone, Copy)]
struct UniqueJson<'a> {
    entries: &'a Cell<usize>,
}

impl UniqueJson<'_> {
    fn charge<E: serde::de::Error>(self) -> Result<(), E> {
        let total = self.entries.get().saturating_add(1);
        if total > MAX_JSON_ENTRIES {
            return Err(E::custom("JSON entry limit exceeded"));
        }
        self.entries.set(total);
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for UniqueJson<'_> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for UniqueJson<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON with unique object keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut value = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            self.charge()?;
            if value.contains_key(&key) {
                return Err(A::Error::custom("duplicate object key"));
            }
            value.insert(key, map.next_value_seed(self)?);
        }
        Ok(Value::Object(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut value = Vec::new();
        while let Some(item) = sequence.next_element_seed(self)? {
            self.charge()?;
            value.push(item);
        }
        Ok(Value::Array(value))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite number"))
    }
    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }
    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_some<D: serde::Deserializer<'de>>(self, value: D) -> Result<Value, D::Error> {
        self.deserialize(value)
    }
}

pub(super) fn parse_unique_json(bytes: &[u8]) -> Result<Value, Error> {
    let entries = Cell::new(0);
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = UniqueJson { entries: &entries }
        .deserialize(&mut deserializer)
        .map_err(|_| Error::Malformed)?;
    deserializer.end().map_err(|_| Error::Malformed)?;
    Ok(value)
}
