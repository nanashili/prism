use crate::error::Result;
use serde::Deserializer;
use serde::de::{DeserializeOwned, Error, SeqAccess, Visitor};
use std::fmt;
use std::marker::PhantomData;

/// Decode an array incrementally, preserving errors from the row consumer.
/// Consumers that persist rows must roll back if this operation fails.
pub(crate) fn parse<T: DeserializeOwned>(
    text: &str,
    consume: impl FnMut(T) -> Result<()>,
) -> Result<()> {
    decode(serde_json::Deserializer::from_str(text), consume)
}

/// Decode an array without first retaining the complete encoded input.
pub(crate) fn parse_reader<T: DeserializeOwned>(
    reader: impl std::io::Read,
    consume: impl FnMut(T) -> Result<()>,
) -> Result<()> {
    decode(serde_json::Deserializer::from_reader(reader), consume)
}

fn decode<'de, T: DeserializeOwned, R: serde_json::de::Read<'de>>(
    mut decoder: serde_json::Deserializer<R>,
    consume: impl FnMut(T) -> Result<()>,
) -> Result<()> {
    let mut visitor = Elements {
        consume,
        failure: None,
        item: PhantomData,
    };
    let decoded = decoder.deserialize_seq(&mut visitor);
    if let Some(error) = visitor.failure {
        return Err(error);
    }
    decoded?;
    decoder.end()?;
    Ok(())
}

struct Elements<T, F> {
    consume: F,
    failure: Option<Box<dyn std::error::Error + Send + Sync>>,
    item: PhantomData<fn() -> T>,
}

impl<'de, T, F> Visitor<'de> for &mut Elements<T, F>
where
    T: DeserializeOwned,
    F: FnMut(T) -> Result<()>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON array")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut array: A) -> std::result::Result<(), A::Error> {
        while let Some(item) = array.next_element()? {
            if let Err(error) = (self.consume)(item) {
                self.failure = Some(error);
                return Err(A::Error::custom("Unable to import a JSON array element"));
            }
        }
        Ok(())
    }
}
