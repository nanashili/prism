use crate::error::Result;
use crate::protocol::{Framing, Rpc};
use crate::settings::AnalysisBackend;
use serde_json::json;
use std::process::Command;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

pub(super) fn connect(
    command: &mut Command,
    backend: AnalysisBackend,
    cancelled: Arc<AtomicBool>,
) -> Result<Rpc> {
    #[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
    let framing = if backend.retains_prepared_session() {
        Framing::ReaLines
    } else {
        Framing::Lines
    };
    #[cfg(not(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra")))]
    let framing = {
        backend.require_available()?;
        Framing::Lines
    };

    let mut rpc = Rpc::spawn(command, framing, cancelled)?;
    rpc.call(
        "initialize",
        json!({
            "protocolVersion":"2024-11-05", "capabilities":{},
            "clientInfo":{"name":"prism","version":env!("CARGO_PKG_VERSION")}
        }),
        Duration::from_secs(30),
        |_| Ok(None),
    )?;
    rpc.notify("notifications/initialized", json!({}))?;
    Ok(rpc)
}
