use super::{Policy, notice, request::Request};
use crate::agent::permission_policy;
use crate::error::{Result, failure};
use crate::events::Sink;
use crate::protocol::Rpc;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_PENDING: usize = 8;
const MAX_ID_BYTES: usize = 1024;
const MAX_TITLE_BYTES: usize = 4096;

#[derive(Default)]
pub(in crate::agent) struct Requests {
    waiting: VecDeque<Request>,
}

impl Requests {
    pub fn admit(
        &mut self,
        id: Value,
        params: &Value,
        policy: &Policy,
        rpc: &mut Rpc,
        sink: &Sink,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        validate_id(&id)?;
        if self.waiting.iter().any(|request| request.id == id) {
            return Err(failure(
                "The agent reused a pending permission request identifier",
            ));
        }
        if cancelled.load(Ordering::Relaxed) {
            return rpc.reply(&id, permission_policy::cancelled());
        }
        match policy.response(params) {
            Ok(Some(response)) => return rpc.reply(&id, response),
            Ok(None) => {}
            Err(error) => notice(sink, "Couldn't read remembered approvals", error)?,
        }
        if self.waiting.len() >= MAX_PENDING || super::title(params).len() > MAX_TITLE_BYTES {
            return rpc.reply(&id, permission_policy::cancelled());
        }
        self.waiting.push_back(Request::new(id, params, policy));
        self.poll(rpc, sink, cancelled)
    }

    pub fn poll(&mut self, rpc: &mut Rpc, sink: &Sink, cancelled: &AtomicBool) -> Result<()> {
        if cancelled.load(Ordering::Relaxed) {
            return self.cancel_all(rpc, sink);
        }
        if let Some(request) = self.waiting.front()
            && let Some(mut response) = request.response()
        {
            if cancelled.load(Ordering::Relaxed) {
                response = permission_policy::cancelled();
            }
            rpc.reply(&request.id, response.clone())?;
            let request = self
                .waiting
                .pop_front()
                .ok_or("The permission request disappeared")?;
            request.dismiss();
            if let Err(error) = request.remember(&response) {
                notice(sink, "Couldn't remember this approval", error)?;
            }
            sink.repaint();
        }
        if let Some(request) = self.waiting.front_mut() {
            request.present(sink)?;
        }
        Ok(())
    }

    pub fn cancel_all(&mut self, rpc: &mut Rpc, sink: &Sink) -> Result<()> {
        if self.waiting.is_empty() {
            return Ok(());
        }
        self.dismiss_all();
        sink.repaint();
        let mut failed = VecDeque::new();
        let mut errors = Vec::new();
        while let Some(request) = self.waiting.pop_front() {
            request.dismiss();
            if let Err(error) = rpc.reply(&request.id, permission_policy::cancelled()) {
                errors.push(error.to_string());
                failed.push_back(request);
            }
        }
        self.waiting = failed;
        sink.repaint();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(failure(format!(
                "Couldn't cancel permission requests: {}",
                errors.join("; ")
            )))
        }
    }

    pub fn dismiss_all(&self) {
        for request in &self.waiting {
            request.dismiss();
        }
    }
}

fn validate_id(id: &Value) -> Result<()> {
    if id.is_number() || id.as_str().is_some_and(|id| id.len() <= MAX_ID_BYTES) {
        Ok(())
    } else {
        Err(failure(
            "The agent sent an invalid or oversized permission request identifier",
        ))
    }
}
