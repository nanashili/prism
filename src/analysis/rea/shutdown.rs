use super::Rea;
#[cfg(feature = "rea-rust-ghidra")]
use super::reply;
use crate::error::Result;
#[cfg(feature = "rea-rust-ghidra")]
use crate::error::failure;
#[cfg(feature = "rea-rust-ghidra")]
use crate::settings::AnalysisBackend;
#[cfg(feature = "rea-rust-ghidra")]
use serde_json::{Value, json};
#[cfg(feature = "rea-rust-ghidra")]
use std::time::{Duration, Instant};

#[cfg(feature = "rea-rust-ghidra")]
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

impl Rea {
    pub(in crate::analysis) fn close(self) -> Result<()> {
        let mut session = self;

        #[cfg(feature = "rea-rust-ghidra")]
        let result = if session.backend == AnalysisBackend::RustGhidraRea
            && session.healthy
            && session.rpc.is_reusable()
            && session.active_batch.is_none()
            && session.abandoned.is_none()
        {
            session.close_idle_session()
        } else {
            Ok(())
        };
        #[cfg(not(feature = "rea-rust-ghidra"))]
        let result = Ok(());

        crate::error::finish(result, session.rpc.close(), "REA process cleanup")
    }

    #[cfg(feature = "rea-rust-ghidra")]
    fn close_idle_session(&mut self) -> Result<()> {
        let started = Instant::now();
        let id = self
            .rpc
            .begin_call("tools/call", json!({"name":"close_binary","arguments":{}}))?;
        let response = self.wait_for_close(id, started)?;
        let result = reply::parse(response)?.into_result()?;
        if result != json!({"result":null}) {
            return Err(failure(
                "REA did not confirm close_binary without a snapshot",
            ));
        }
        Ok(())
    }

    #[cfg(feature = "rea-rust-ghidra")]
    fn wait_for_close(&mut self, id: u64, started: Instant) -> Result<Value> {
        // Attempt cleanup even when a caller cancelled the last operation.
        // Poll directly to retain that flag and bound acknowledgement waiting.
        loop {
            let remaining = CLOSE_TIMEOUT.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(failure("REA close_binary exceeded its 2 second deadline"));
            }
            let message = match self.rpc.take_reply(id) {
                Some(message) => Some(message),
                None => self.rpc.poll(remaining.min(Duration::from_millis(25)))?,
            };
            let Some(message) = message else {
                continue;
            };
            if message.get("id").and_then(Value::as_u64) == Some(id)
                && message.get("method").is_none()
            {
                return self.rpc.finish_call(message);
            }

            // This lifecycle operation emits notifications without client requests.
            // Reject requests rather than risking another blocking protocol write.
            if message.get("id").is_some() || message.get("method").is_none() {
                return Err(failure("REA sent an unexpected message while closing"));
            }
        }
    }
}
