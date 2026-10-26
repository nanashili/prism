use crate::error::{Result, failure};
use serde::Serialize;
use std::io::{BufRead, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(super) const MAX_MESSAGE: usize = 8 * 1024 * 1024;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(super) fn read_cancellable(
    socket: &mut std::net::TcpStream,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> Result<Vec<u8>> {
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(crate::error::failure("Tool request cancelled"));
        }
        if Instant::now() >= deadline {
            return Err(crate::error::failure("The tool request timed out"));
        }
        match socket.read(&mut buffer) {
            Ok(0) => return Err(crate::error::failure("The window bridge closed")),
            Ok(length) => {
                bytes.extend_from_slice(&buffer[..length]);
                if bytes.len() > MAX_MESSAGE {
                    return Err(crate::error::failure("MCP reply exceeds 8 MiB"));
                }
                if bytes.contains(&b'\n') {
                    return Ok(bytes);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(super) fn read(input: &mut impl BufRead) -> Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    input
        .take((MAX_MESSAGE + 1) as u64)
        .read_until(b'\n', &mut bytes)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > MAX_MESSAGE {
        return Err(failure("MCP message exceeds 8 MiB"));
    }
    Ok(Some(bytes))
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(super) fn write<T: Serialize>(output: &mut impl Write, message: &T) -> Result<()> {
    let bytes = serde_json::to_vec(message)?;
    write_bytes(output, &bytes)
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(super) fn write_bytes(output: &mut impl Write, bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_MESSAGE {
        return Err(failure("MCP reply exceeds 8 MiB"));
    }
    output.write_all(bytes)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
