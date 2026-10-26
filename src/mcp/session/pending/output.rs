use super::error;
use crate::error::{Result, failure};
use crate::mcp::transport;
use serde_json::Value;
use std::io::{self, Write};
use std::sync::Mutex;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(super) fn write(output: &Mutex<impl Write>, response: &Value) -> Result<()> {
    let mut bytes = MessageBuffer(Vec::new());
    if serde_json::to_writer(&mut bytes, response).is_err() {
        bytes.0.clear();
        let error = error(response["id"].clone(), -32603, "Reply exceeds 8 MiB");
        serde_json::to_writer(&mut bytes, &error)?;
    }
    let mut output = output
        .lock()
        .map_err(|_| failure("MCP output closed unexpectedly"))?;
    transport::write_bytes(&mut *output, &bytes.0)
}

struct MessageBuffer(Vec<u8>);

impl Write for MessageBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > transport::MAX_MESSAGE.saturating_sub(self.0.len()) {
            return Err(io::Error::other("Reply exceeds 8 MiB"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
