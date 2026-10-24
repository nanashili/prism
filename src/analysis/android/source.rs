use super::inventory::{self, Discard};
use crate::error::{Result, failure};
use crate::project::files::TEXT_LIMIT;
use serde::Deserializer;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::fmt;
use std::path::Path;

#[cfg(test)]
mod tests;

pub(super) fn code(path: &Path, owner: &str, signature: &str) -> Result<String> {
    let mut reader = inventory::input(path)?;
    let mut decoder = serde_json::Deserializer::from_reader(&mut reader);
    let selected = Source {
        owner,
        signature,
        kind: Kind::Class,
        depth: 0,
    }
    .deserialize(&mut decoder)
    .and_then(|value| decoder.end().map(|()| value));
    inventory::check_limit(&reader)?;
    let selected = selected?;
    if selected.is_null() {
        return Err(failure("The indexed Android class is missing"));
    }
    let method = &selected["method"];
    if method.is_null() {
        return Err(failure("The indexed Android method is missing"));
    }
    let mut text = format!(
        "{} {{\n",
        method["declaration"].as_str().unwrap_or(signature)
    );
    let lines = &method["lines"];
    let body = lines["body"].as_str().unwrap_or_default();
    if lines["overflow"] == true
        || (lines["has_lines"] == true && text.len() + body.len() > TEXT_LIMIT)
    {
        return Err(failure("This method exceeds the code display limit"));
    }
    text.push_str(body);
    text.push_str("}\n");
    Ok(text)
}

#[derive(Clone, Copy)]
enum Kind {
    Class,
    Classes,
    Methods,
    Method,
    Lines,
    Line,
    Owner,
    Signature,
    Declaration,
    Code,
}

#[derive(Clone, Copy)]
struct Source<'a> {
    owner: &'a str,
    signature: &'a str,
    kind: Kind,
    depth: usize,
}

impl Source<'_> {
    fn with(self, kind: Kind) -> Self {
        Self { kind, ..self }
    }
}

impl<'de> DeserializeSeed<'de> for Source<'_> {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> std::result::Result<Value, D::Error> {
        decoder.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Source<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("valid JADX source JSON")
    }

    fn visit_unit<E>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E>(self, _: bool) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_i64<E>(self, _: i64) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_u64<E>(self, _: u64) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_f64<E>(self, _: f64) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_str<E>(self, text: &str) -> std::result::Result<Value, E> {
        Ok(match self.kind {
            Kind::Owner => (text == self.owner).into(),
            Kind::Signature => (text == self.signature).into(),
            Kind::Declaration => text.into(),
            // A single oversized line already exceeds the existing method
            // display limit. Retain its overflow state, never its whole body.
            Kind::Code if text.len() > TEXT_LIMIT => Value::Bool(true),
            Kind::Code => text.into(),
            _ => Value::Null,
        })
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> std::result::Result<Value, A::Error> {
        match self.kind {
            Kind::Classes | Kind::Methods => {
                let child = if matches!(self.kind, Kind::Classes) {
                    Self {
                        kind: Kind::Class,
                        depth: self.depth + 1,
                        ..self
                    }
                } else {
                    self.with(Kind::Method)
                };
                let mut selected = Value::Null;
                while let Some(value) = sequence.next_element_seed(child)? {
                    if !value.is_null() {
                        selected = value;
                        // The legacy lookup chooses the first match, but still
                        // parses and validates the entire input document.
                        while sequence.next_element::<Discard>()?.is_some() {}
                        break;
                    }
                }
                Ok(selected)
            }
            Kind::Lines => {
                let mut body = String::new();
                let mut has_lines = false;
                let mut overflow = false;
                while let Some(line) = sequence.next_element_seed(self.with(Kind::Line))? {
                    has_lines = true;
                    let code = line.as_str().unwrap_or_default();
                    if line == true || body.len() + code.len() + 5 > TEXT_LIMIT {
                        overflow = true;
                        body.clear();
                        while sequence.next_element::<Discard>()?.is_some() {}
                        break;
                    }
                    body.push_str("    ");
                    body.push_str(code);
                    body.push('\n');
                }
                Ok(object([
                    ("body", Value::String(body)),
                    ("has_lines", has_lines.into()),
                    ("overflow", overflow.into()),
                ]))
            }
            _ => {
                while sequence.next_element::<Discard>()?.is_some() {}
                Ok(Value::Null)
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Value, A::Error> {
        let mut matches = false;
        let mut method = Value::Null;
        let mut inner = Value::Null;
        let mut declaration = Value::Null;
        let mut lines = Value::Null;
        let mut code = Value::Null;
        while let Some(key) = map.next_key::<String>()? {
            match (self.kind, key.as_str()) {
                (Kind::Class, "name") if self.depth <= 64 => {
                    matches = map.next_value_seed(self.with(Kind::Owner))? == true;
                }
                (Kind::Class, "methods") if self.depth <= 64 => {
                    method = map.next_value_seed(self.with(Kind::Methods))?;
                }
                (Kind::Class, "inner-classes") if self.depth <= 64 => {
                    inner = map.next_value_seed(self.with(Kind::Classes))?;
                }
                (Kind::Method, "signature") => {
                    matches = map.next_value_seed(self.with(Kind::Signature))? == true;
                }
                (Kind::Method, "declaration") => {
                    declaration = map.next_value_seed(self.with(Kind::Declaration))?;
                }
                (Kind::Method, "lines") => {
                    lines = map.next_value_seed(self.with(Kind::Lines))?;
                }
                (Kind::Line, "code") => {
                    code = map.next_value_seed(self.with(Kind::Code))?;
                }
                _ => {
                    map.next_value::<Discard>()?;
                }
            }
        }
        Ok(match self.kind {
            // A matching parent's missing method must not fall through to an
            // inner class with the same name. Field order cannot change this.
            Kind::Class if matches => object([("method", method)]),
            Kind::Class => inner,
            Kind::Method if matches => object([("declaration", declaration), ("lines", lines)]),
            Kind::Line => code,
            _ => Value::Null,
        })
    }
}

fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}
