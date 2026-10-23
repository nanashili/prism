use super::Rea;
use super::batch_state::PendingCall;
use crate::error::{Result, failure};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

impl Rea {
    pub(super) fn ensure_available(&mut self) -> Result<()> {
        if !self.healthy {
            return Err(failure(
                "The prepared Ghidra session is unavailable. Reopen the project to prepare it again.",
            ));
        }
        if let Some(call) = self.abandoned.take() {
            match self.poll_call(&call) {
                Ok(Some(value)) => {
                    // The caller abandoned this result. Only session loss affects the next call.
                    if let super::reply::ToolReply::Failure(error) = self.parse_tool_reply(value)?
                        && error.session_unavailable()
                    {
                        return Err(error.into_error());
                    }
                }
                Ok(None) => {
                    self.abandoned = Some(call);
                    return Err(failure(
                        "The previous Ghidra batch is still finishing. Wait before resuming.",
                    ));
                }
                Err(error) => {
                    self.healthy = false;
                    return Err(error);
                }
            }
        }
        if self.active_batch.is_some() {
            return Err(failure(
                "A Ghidra decompilation batch is active. Pause it before making another analysis request.",
            ));
        }
        Ok(())
    }

    pub(super) fn begin_tool(&mut self, name: &str, arguments: Value) -> Result<PendingCall> {
        let id = self
            .rpc
            .begin_call("tools/call", json!({"name":name,"arguments":arguments}))?;
        Ok(PendingCall {
            id,
            started: Instant::now(),
        })
    }

    pub(super) fn poll_call(&mut self, call: &PendingCall) -> Result<Option<Value>> {
        if self.rpc.is_cancelled() {
            return Err(failure("Cancelled"));
        }
        if call.started.elapsed() > self.timeout {
            return Err(failure(
                "Ghidra batch timed out. Reopen the project before resuming.",
            ));
        }
        for _ in 0..32 {
            let message = match self.rpc.take_reply(call.id) {
                Some(message) => Some(message),
                None => self.rpc.poll(Duration::ZERO)?,
            };
            let Some(message) = message else {
                return Ok(None);
            };
            if message.get("id").and_then(Value::as_u64) == Some(call.id)
                && message.get("method").is_none()
            {
                return self.rpc.finish_call(message).map(Some);
            }
            let sink = self.sink.clone();
            self.rpc
                .dispatch(message, |message| super::progress(&sink, message))?;
        }
        Ok(None)
    }
}
