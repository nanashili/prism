use crate::error::{Result, failure};
use serde_json::Value;
use std::io::{BufRead, Read, Write};

const MAX_FRAME: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) enum Framing {
    Lines,
    #[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
    ReaLines,
    Dap,
    Zero,
}

impl Framing {
    pub(super) fn read(self, input: &mut impl BufRead) -> Result<Option<Frame>> {
        let limit = match self {
            Self::Zero => 32 * 1024 * 1024,
            #[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
            Self::ReaLines => 10 * 1024 * 1024,
            _ => MAX_FRAME,
        };
        let bytes = match self {
            Self::Dap => match self.read_dap(input)? {
                Some(bytes) => bytes,
                None => return Ok(None),
            },
            _ => {
                let mut line = Vec::new();
                let length = input.take((limit + 1) as u64).read_until(
                    if matches!(self, Self::Zero) { 0 } else { b'\n' },
                    &mut line,
                )?;
                if length == 0 {
                    return Ok(None);
                }
                if length > limit {
                    return Err(failure(format!(
                        "The server response exceeded {} MiB",
                        limit / (1024 * 1024)
                    )));
                }
                line
            }
        };
        Ok(Some(Frame {
            bytes,
            framing: self,
        }))
    }

    fn read_dap(self, input: &mut impl BufRead) -> Result<Option<Vec<u8>>> {
        let mut length = None;
        let mut header_bytes = 0;
        loop {
            let mut line = String::new();
            let count = input.take(4097).read_line(&mut line)?;
            header_bytes += count;
            if count == 0 {
                return Ok(None);
            }
            if header_bytes > 4096 {
                return Err(failure("Invalid debugger header"));
            }
            if line.trim().is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                length = Some(value.trim().parse::<usize>()?);
            }
        }
        let length = length.ok_or("The debugger omitted Content-Length")?;
        if length > MAX_FRAME {
            return Err(failure("The debugger response exceeded 8 MiB"));
        }
        let mut bytes = vec![0; length];
        input.read_exact(&mut bytes)?;
        Ok(Some(bytes))
    }

    pub fn write(self, output: &mut impl Write, value: &Value) -> Result<()> {
        let bytes = if matches!(self, Self::Zero) {
            value
                .as_str()
                .ok_or("A Rizin command must be text")?
                .as_bytes()
                .to_vec()
        } else {
            serde_json::to_vec(value)?
        };
        if bytes.len() > MAX_FRAME {
            return Err(failure("The outgoing request exceeds 8 MiB"));
        }
        match self {
            Self::Dap => {
                write!(output, "Content-Length: {}\r\n\r\n", bytes.len())?;
                output.write_all(&bytes)?;
            }
            _ => {
                output.write_all(&bytes)?;
                output.write_all(b"\n")?;
            }
        }
        output.flush()?;
        Ok(())
    }
}

pub(super) struct Frame {
    bytes: Vec<u8>,
    framing: Framing,
}

impl Frame {
    pub(super) fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    pub(super) fn decode(mut self) -> Result<Value> {
        if matches!(self.framing, Framing::Zero) {
            if self.bytes.last() == Some(&0) {
                self.bytes.pop();
            }
            return Ok(serde_json::from_slice(&self.bytes).unwrap_or_else(|_| {
                Value::String(match String::from_utf8(self.bytes) {
                    Ok(text) => text,
                    Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
                })
            }));
        }
        Ok(serde_json::from_slice(&self.bytes)?)
    }
}

#[cfg(test)]
mod tests;
