use super::Session;
use crate::error::{Result, failure};
use crate::events::Sink;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

impl Session {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn call(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
        sink: &Sink,
        cancelled: &AtomicBool,
        controls: &Receiver<super::Control>,
    ) -> Result<Value> {
        let result = self.exchange(method, params, timeout, sink, cancelled, controls);
        if result.is_err() {
            self.finish_permissions(result, sink)
        } else {
            result
        }
    }

    fn exchange(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
        sink: &Sink,
        cancelled: &AtomicBool,
        controls: &Receiver<super::Control>,
    ) -> Result<Value> {
        let id = self.rpc.begin_call(method, params)?;
        let started = Instant::now();
        loop {
            if self.rpc.is_cancelled() || cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            if started.elapsed() > timeout {
                return Err(failure(format!("{method} timed out")));
            }
            self.poll_permissions(sink, cancelled)?;
            if !self.id.is_empty() {
                self.send_controls(controls, sink)?;
            }
            let Some(mut message) = self.rpc.poll(Duration::from_millis(100))? else {
                continue;
            };
            if crate::protocol::response_id(&message) == Some(id) {
                return self.rpc.finish_call(message);
            }
            if !self.control_reply(&mut message, sink)? {
                self.receive(message, sink, cancelled)?;
            }
        }
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn receive(
        &mut self,
        message: Value,
        sink: &Sink,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        // Late replies to expired controls have no remaining consumer.
        if crate::protocol::response_id(&message).is_some() {
            return Ok(());
        }
        if message["method"] == "session/request_permission" {
            let Some(id) = message.get("id") else {
                return Ok(());
            };
            return self.permission_requests.admit(
                id.clone(),
                &message["params"],
                &self.permissions,
                &mut self.rpc,
                sink,
                cancelled,
            );
        }
        self.rpc.dispatch(message, |message| {
            if message["method"] == "session/update" {
                self.updates.receive(&message["params"], sink)?;
            }
            Ok(None)
        })
    }

    pub(super) fn poll_permissions(&mut self, sink: &Sink, cancelled: &AtomicBool) -> Result<()> {
        self.permission_requests
            .poll(&mut self.rpc, sink, cancelled)
    }

    pub(super) fn finish_permissions<T>(&mut self, result: Result<T>, sink: &Sink) -> Result<T> {
        if !self.rpc.is_reusable() {
            self.permission_requests.dismiss_all();
            sink.repaint();
            return result;
        }
        let cleanup = self.permission_requests.cancel_all(&mut self.rpc, sink);
        match (result, cleanup) {
            (Ok(result), Ok(())) => Ok(result),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(cleanup)) => Err(failure(format!(
                "{error}; permission cleanup failed: {cleanup}"
            ))),
        }
    }
}
