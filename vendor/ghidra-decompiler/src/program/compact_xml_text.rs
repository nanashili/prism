use crate::error::{Error, Result};

use super::parse::invalid_utf8;

pub(super) fn validate_source(source: &[u8]) -> Result<()> {
    if source.len() > 512 * 1024 * 1024 {
        return Err(Error::Decoder("saved XML exceeds the 512 MiB input limit".into()));
    }
    let text = std::str::from_utf8(source).map_err(invalid_utf8)?;
    validate_characters(text)
}

pub(super) fn validate_text(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).map_err(invalid_utf8)?;
    let unescaped =
        quick_xml::escape::unescape(text).map_err(|error| Error::Decoder(format!("invalid XML entity: {error}")))?;
    validate_characters(&unescaped)
}

fn validate_characters(text: &str) -> Result<()> {
    if text.chars().all(|character| {
        matches!(character,
        '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
    }) {
        Ok(())
    } else {
        Err(Error::Decoder("invalid XML character".into()))
    }
}

pub(super) fn is_whitespace(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
}

// Snapshot tag/attribute names are identifiers emitted by the native encoder.
// User-visible names are values and keep their full Unicode support.
pub(super) fn validate_name(bytes: &[u8]) -> Result<()> {
    let first = bytes
        .first()
        .copied()
        .ok_or_else(|| Error::Decoder("empty XML name".into()))?;
    let valid = |byte: u8| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b':');
    if !valid(first)
        || !bytes[1..]
            .iter()
            .all(|byte| valid(*byte) || byte.is_ascii_digit() || matches!(byte, b'.' | b'-'))
    {
        return Err(Error::Decoder("invalid saved XML identifier".into()));
    }
    Ok(())
}
