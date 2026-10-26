use crate::error::{Result, failure};
use prism::mcp::{self, CallClass, Mode};
use serde_json::{Value, json};
use std::io::{self, Read};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MAX_MESSAGE: usize = 8 * 1024 * 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) struct Incoming {
    pub(super) tool: String,
    pub(super) args: Value,
    pub(super) chat: Option<u64>,
    pub(super) class: CallClass,
    pub(super) deadline: Instant,
    pub(super) bytes: usize,
}

impl Incoming {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn read(
        socket: &mut TcpStream,
        token: &str,
        mode: Mode,
        stopped: &AtomicBool,
    ) -> Result<Self> {
        let started = Instant::now();
        let bytes = read_frame(socket, stopped, started + HANDSHAKE_TIMEOUT)?;
        let mut request: Value = serde_json::from_slice(&bytes)?;
        if request["token"].as_str() != Some(token) {
            return Err(failure("Invalid bridge token"));
        }
        mcp::validate_call(mode, &request["params"]).map_err(failure)?;

        let params = request.get_mut("params").ok_or("Missing tool parameters")?;
        let tool = params["name"]
            .as_str()
            .ok_or("Missing tool name")?
            .to_owned();
        let args = params
            .as_object_mut()
            .and_then(|params| params.remove("arguments"))
            .unwrap_or_else(|| json!({}));
        let class = mcp::classify_call(&tool, &args);
        let timeout = match request.get("timeout_ms") {
            Some(value) => Duration::from_millis(value.as_u64().ok_or("Invalid tool timeout")?)
                .min(class.timeout()),
            None => class.timeout(),
        };
        let chat = request
            .get("chat")
            .map(|chat| chat.as_u64().ok_or("Invalid chat identity"))
            .transpose()?;

        Ok(Self {
            tool,
            args,
            chat,
            class,
            deadline: started + timeout,
            bytes: bytes.len(),
        })
    }
}

fn read_frame(socket: &mut TcpStream, stopped: &AtomicBool, deadline: Instant) -> Result<Vec<u8>> {
    socket.set_nonblocking(true)?;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];

    loop {
        if stopped.load(Ordering::Relaxed) {
            return Err(failure("Prism is closing"));
        }
        if Instant::now() >= deadline {
            return Err(failure(
                "The bridge request was not received within two seconds",
            ));
        }

        match socket.read(&mut buffer) {
            Ok(0) if bytes.is_empty() => return Err(failure("Empty bridge request")),
            Ok(0) => return Ok(bytes),
            Ok(length) => {
                if length > MAX_MESSAGE.saturating_sub(bytes.len()) {
                    return Err(failure("The bridge request exceeds 8 MiB"));
                }
                let end = buffer[..length].iter().position(|byte| *byte == b'\n');
                if let Some(index) = end {
                    bytes.extend_from_slice(&buffer[..=index]);
                    if index + 1 != length {
                        return Err(failure("Unexpected data after the tool request"));
                    }
                    return Ok(bytes);
                }
                bytes.extend_from_slice(&buffer[..length]);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::read_frame;
    use crate::error::Result;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::{Duration, Instant};

    fn connected_pair() -> Result<(TcpStream, TcpStream)> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let address = listener.local_addr()?;
        let client = TcpStream::connect(address)?;
        let (server, _) = listener.accept()?;
        Ok((server, client))
    }

    #[test]
    fn rejects_data_after_the_request_delivered_in_the_same_read() -> Result<()> {
        let (mut server, mut client) = connected_pair()?;
        let sender = thread::spawn(move || client.write_all(b"{}\ntrailing"));

        let error = read_frame(
            &mut server,
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(1),
        )
        .expect_err("trailing bytes must be rejected");
        sender.join().expect("client sender should finish")?;
        assert!(error.to_string().contains("Unexpected data"));
        Ok(())
    }
}
