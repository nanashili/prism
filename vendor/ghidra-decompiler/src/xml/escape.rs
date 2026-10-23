/// Yield unchanged UTF-8 runs and XML entities without allocating a second string.
pub(crate) fn escaped_xml(text: &str) -> impl Iterator<Item = &str> {
    EscapedXml {
        remaining: text,
        pending: None,
    }
}

struct EscapedXml<'a> {
    remaining: &'a str,
    pending: Option<&'static str>,
}

impl<'a> Iterator for EscapedXml<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entity) = self.pending.take() {
            return Some(entity);
        }
        if self.remaining.is_empty() {
            return None;
        }

        let Some(index) = self.remaining.bytes().position(|byte| entity(byte).is_some()) else {
            return Some(std::mem::take(&mut self.remaining));
        };
        let escaped = entity(self.remaining.as_bytes()[index])?;
        let unchanged = &self.remaining[..index];
        self.remaining = &self.remaining[index + 1..];
        if unchanged.is_empty() {
            Some(escaped)
        } else {
            self.pending = Some(escaped);
            Some(unchanged)
        }
    }
}

fn entity(byte: u8) -> Option<&'static str> {
    match byte {
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        b'&' => Some("&amp;"),
        b'\"' => Some("&quot;"),
        b'\'' => Some("&apos;"),
        _ => None,
    }
}
