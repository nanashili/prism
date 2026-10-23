use super::{Rea, reply};
use crate::error::{Result, failure};
use crate::settings::AnalysisBackend;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

impl Drop for Rea {
    fn drop(&mut self) {
        if self.backend != AnalysisBackend::RustGhidraRea
            || !self.healthy
            || !self.rpc.is_reusable()
            || self.active_batch.is_some()
            || self.abandoned.is_some()
        {
            return;
        }

        if let Err(error) = self.close_idle_session() {
            eprintln!("Cannot finish closing the Rust REA session: {error}");
        }
    }
}

impl Rea {
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
