use serde_json::Value;
use std::io::{self, Write};

pub(crate) fn json_preview(value: &Value, limit: usize) -> String {
    if let Some(text) = value.as_str() {
        return shortened(text.as_bytes(), limit, text.len() > limit);
    }
    let mut preview = Preview {
        bytes: Vec::new(),
        limit,
        full: false,
    };
    let encoded = serde_json::to_writer(&mut preview, value);
    match encoded {
        Ok(()) => shortened(&preview.bytes, limit, false),
        Err(_) if preview.full => shortened(&preview.bytes, limit, true),
        Err(error) => format!("[Could not preview input: {error}]"),
    }
}

struct Preview {
    bytes: Vec<u8>,
    limit: usize,
    full: bool,
}

impl Write for Preview {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.bytes.len());
        self.bytes
            .extend_from_slice(&bytes[..bytes.len().min(remaining)]);
        if bytes.len() > remaining {
            self.full = true;
            return Err(io::Error::other("The input preview reached its limit"));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn shortened(bytes: &[u8], limit: usize, truncated: bool) -> String {
    let bytes = &bytes[..bytes.len().min(limit)];
    let valid = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => std::str::from_utf8(&bytes[..error.valid_up_to()]).unwrap_or_default(),
    };
    let mut result = valid.to_owned();
    if truncated {
        result.push_str("\n[Preview shortened]");
    }
    result
}
