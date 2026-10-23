use crate::error::{Result, failure};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;
use std::io::{BufReader, Read, Take};
use std::path::Path;

const JSON_LIMIT: u64 = 16 * 1024 * 1024;

pub(super) fn read(path: &Path) -> Result<Value> {
    let mut reader = input(path)?;
    // from_reader also checks EOF/trailing data. The capped reader remains
    // necessary if the file grows after its metadata was read.
    let parsed = serde_json::from_reader::<_, Inventory>(&mut reader);
    check_limit(&reader)?;
    Ok(parsed?.0)
}

pub(super) fn input(path: &Path) -> Result<BufReader<Take<std::fs::File>>> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > JSON_LIMIT {
        return Err(failure("The Android metadata exceeds 16 MiB"));
    }
    Ok(BufReader::with_capacity(
        64 * 1024,
        file.take(JSON_LIMIT + 1),
    ))
}

pub(super) fn check_limit(reader: &BufReader<Take<std::fs::File>>) -> Result<()> {
    if reader.get_ref().limit() == 0 {
        return Err(failure("The Android metadata exceeds 16 MiB"));
    }
    Ok(())
}

struct Inventory(Value);

pub(super) struct Discard;

impl<'de> Deserialize<'de> for Discard {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Metadata { retain: false }
            .deserialize(deserializer)
            .map(|_| Self)
    }
}

impl<'de> Deserialize<'de> for Inventory {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Metadata { retain: true }
            .deserialize(deserializer)
            .map(Self)
    }
}

// Keep the existing inventory consumer's missing/null/type fallback and
// duplicate-key semantics without retaining source lines or annotations.
struct Metadata {
    retain: bool,
}

impl<'de> DeserializeSeed<'de> for Metadata {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Metadata {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("valid JADX JSON metadata")
    }

    fn visit_unit<E>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E>(self, value: bool) -> std::result::Result<Value, E> {
        Ok(if self.retain {
            Value::Bool(value)
        } else {
            Value::Null
        })
    }

    fn visit_i64<E>(self, value: i64) -> std::result::Result<Value, E> {
        Ok(if self.retain {
            Value::Number(value.into())
        } else {
            Value::Null
        })
    }

    fn visit_u64<E>(self, value: u64) -> std::result::Result<Value, E> {
        Ok(if self.retain {
            Value::Number(value.into())
        } else {
            Value::Null
        })
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> std::result::Result<Value, E> {
        let number = Number::from_f64(value).ok_or_else(|| E::custom("Invalid JSON number"))?;
        Ok(if self.retain {
            Value::Number(number)
        } else {
            Value::Null
        })
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Value, E> {
        Ok(if self.retain {
            Value::String(value.into())
        } else {
            Value::Null
        })
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> std::result::Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(Metadata {
            retain: self.retain,
        })? {
            if self.retain {
                values.push(value);
            }
        }
        Ok(if self.retain {
            Value::Array(values)
        } else {
            Value::Null
        })
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key_seed(Key)? {
            let retain = self.retain && key.is_some();
            // Deserialize discarded values too, rather than IgnoredAny: this
            // preserves UTF-8, number and recursion-limit validation everywhere.
            let value = map.next_value_seed(Metadata { retain })?;
            if self.retain
                && let Some(key) = key
            {
                values.insert(key.into(), value);
            }
        }
        Ok(if self.retain {
            Value::Object(values)
        } else {
            Value::Null
        })
    }
}

struct Key;

impl<'de> DeserializeSeed<'de> for Key {
    type Value = Option<&'static str>;

    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for Key {
    type Value = Option<&'static str>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object key")
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E> {
        Ok(match value {
            "classes" => Some("classes"),
            "inner" => Some("inner"),
            "json" => Some("json"),
            "name" => Some("name"),
            "dex" => Some("dex"),
            "methods" => Some("methods"),
            "inner-classes" => Some("inner-classes"),
            "offset" => Some("offset"),
            "signature" => Some("signature"),
            _ => None,
        })
    }
}
