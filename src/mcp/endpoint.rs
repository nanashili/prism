use super::transport;
use crate::error::{Result, failure};
use serde::Serialize;
use serde_json::Value;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct Endpoint {
    address: SocketAddr,
    token: String,
}

#[derive(Serialize)]
struct WireRequest<'a> {
    token: &'a str,
    params: &'a Value,
    timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    chat: Option<u64>,
}

pub fn support_dir() -> PathBuf {
    std::env::var_os("PRISM_SUPPORT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("Prism")
        })
}

impl Endpoint {
    pub(super) fn new(address: &str, token: &str) -> Result<Self> {
        let address: SocketAddr = address.parse()?;
        if !address.ip().is_loopback() {
            return Err(failure("The bridge must be on this computer"));
        }
        if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(failure("Invalid window token"));
        }
        Ok(Self {
            address,
            token: token.into(),
        })
    }

    pub(super) fn discover() -> Result<Self> {
        let file = std::fs::File::open(support_dir().join("window.json"))
            .map_err(|error| failure(format!("Open Prism first. Window address: {error}")))?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > 4096 {
            return Err(failure("Invalid window address file"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(failure(
                    "The window address file must be private to your user",
                ));
            }
        }
        let endpoint: Value = serde_json::from_reader(file)?;
        Self::new(
            endpoint["address"]
                .as_str()
                .ok_or("Missing window address")?,
            endpoint["token"].as_str().ok_or("Missing window token")?,
        )
    }

    pub(super) fn call_cancellable(
        &self,
        params: &Value,
        cancelled: &std::sync::atomic::AtomicBool,
        deadline: Instant,
    ) -> Result<Value> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) || remaining.is_zero() {
            return Err(failure("The tool request was cancelled or timed out"));
        }
        let connect_timeout = remaining.min(Duration::from_secs(3));
        let mut socket = TcpStream::connect_timeout(&self.address, connect_timeout)
            .map_err(|error| failure(format!("Cannot reach Prism. Open it and retry: {error}")))?;
        socket.set_write_timeout(Some(connect_timeout))?;
        let chat = if params["name"] == super::question::TOOL {
            std::env::var_os("PRISM_CHAT_ID")
                .map(|chat| -> Result<u64> {
                    Ok(chat
                        .to_str()
                        .ok_or("Invalid chat identity")?
                        .parse::<u64>()?)
                })
                .transpose()?
        } else {
            None
        };
        let request = WireRequest {
            token: &self.token,
            params,
            timeout_ms: u64::try_from(
                deadline
                    .saturating_duration_since(Instant::now())
                    .as_millis(),
            )?,
            chat,
        };
        transport::write(&mut socket, &request)?;
        let bytes = transport::read_cancellable(&mut socket, cancelled, deadline)?;
        let mut response: Value = serde_json::from_slice(&bytes)?;
        if let Some(error) = response["error"].as_str() {
            return Err(failure(error));
        }
        response
            .get_mut("result")
            .map(Value::take)
            .ok_or_else(|| failure("Invalid window reply"))
    }
}
