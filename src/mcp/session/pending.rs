use super::{Endpoint, Mode, error, reply, tool_result};
use crate::error::{Result, failure};
use crate::mcp::{CallClass, classify_call, question, validate_call};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::time::Instant;

mod output;

pub(super) struct Pending<W: Write + Send + 'static> {
    output: Arc<Mutex<W>>,
    requests: BTreeMap<String, Request>,
}

struct Request {
    cancelled: Arc<AtomicBool>,
    class: CallClass,
    bytes: usize,
    thread: JoinHandle<Result<()>>,
}

impl<W: Write + Send + 'static> Pending<W> {
    pub(super) fn new(output: W) -> Self {
        Self {
            output: Arc::new(Mutex::new(output)),
            requests: BTreeMap::new(),
        }
    }

    pub(super) fn has_id(&self, id: &Value) -> bool {
        !id.is_null() && self.requests.contains_key(&id.to_string())
    }

    pub(super) fn accept(
        &mut self,
        request: &mut Value,
        bytes: usize,
        mode: Mode,
        endpoint: Option<&Endpoint>,
    ) -> Result<bool> {
        if request["method"] == "notifications/cancelled" && request.get("id").is_none() {
            if let Some(pending) = self
                .requests
                .get(&request["params"]["requestId"].to_string())
            {
                pending.cancelled.store(true, Ordering::Relaxed);
            }
            return Ok(true);
        }
        if request["method"] != "tools/call" || request.get("id").is_none() {
            return Ok(false);
        }
        let id = &request["id"];
        let params = &request["params"];
        let validation = validate_call(mode, params).and_then(|()| {
            if params["name"] == question::TOOL {
                question::Question::parse(params["arguments"].clone()).map(|_| ())
            } else {
                Ok(())
            }
        });
        if let Err(message) = validation {
            self.write(&error(id.clone(), -32602, &message))?;
            return Ok(true);
        }
        let class = classify_call(
            params["name"].as_str().unwrap_or_default(),
            &params["arguments"],
        );
        if !self.has_capacity(class, bytes) {
            self.write(&error(
                id.clone(),
                -32000,
                "Too many pending tool requests in this class",
            ))?;
            return Ok(true);
        }
        let id = request["id"].take();
        let params = request["params"].take();
        self.start(id, params, endpoint.cloned(), class, bytes)?;
        Ok(true)
    }

    fn has_capacity(&self, class: CallClass, bytes: usize) -> bool {
        let mut count = 0;
        let mut used = 0;
        for request in self
            .requests
            .values()
            .filter(|request| request.class == class)
        {
            count += 1;
            used += request.bytes;
        }
        count < class.capacity() && bytes <= class.byte_budget().saturating_sub(used)
    }

    pub(super) fn write(&self, response: &Value) -> Result<()> {
        output::write(&self.output, response)
    }

    fn start(
        &mut self,
        id: Value,
        params: Value,
        endpoint: Option<Endpoint>,
        class: CallClass,
        bytes: usize,
    ) -> Result<()> {
        let output = self.output.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let deadline = Instant::now() + class.timeout();
        let key = id.to_string();
        let thread = std::thread::Builder::new()
            .name("prism-mcp-request".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                let result = endpoint
                    .map(Ok)
                    .unwrap_or_else(Endpoint::discover)
                    .and_then(|endpoint| endpoint.call_cancellable(&params, &flag, deadline));
                let result = match result {
                    Ok(value) => tool_result(value),
                    Err(error) => {
                        json!({"isError":true,"content":[{"type":"text","text":error.to_string()}]})
                    }
                };
                output::write(&output, &reply(id, result))
            })?;
        self.requests.insert(
            key,
            Request {
                cancelled,
                class,
                bytes,
                thread,
            },
        );
        Ok(())
    }

    pub(super) fn reap(&mut self) -> Result<()> {
        let completed: Vec<_> = self
            .requests
            .iter()
            .filter(|(_, request)| request.thread.is_finished())
            .map(|(key, _)| key.clone())
            .collect();
        let mut failures = Vec::new();
        for key in completed {
            if let Some(request) = self.requests.remove(&key) {
                collect_failure(request.thread, &mut failures);
            }
        }
        joined(failures)
    }

    pub(super) fn finish(mut self, primary: Result<()>) -> Result<()> {
        match (primary, self.close()) {
            (Err(primary), Err(cleanup)) => {
                Err(failure(format!("{primary}\nMCP shutdown: {cleanup}")))
            }
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Ok(()), Ok(())) => Ok(()),
        }
    }

    fn close(&mut self) -> Result<()> {
        for request in self.requests.values() {
            request.cancelled.store(true, Ordering::Relaxed);
        }
        let mut failures = Vec::new();
        for (_, request) in std::mem::take(&mut self.requests) {
            collect_failure(request.thread, &mut failures);
        }
        joined(failures)
    }
}

impl<W: Write + Send + 'static> Drop for Pending<W> {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("Closing MCP requests failed: {error}");
        }
    }
}

fn collect_failure(thread: JoinHandle<Result<()>>, failures: &mut Vec<String>) {
    match thread.join() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => failures.push(error.to_string()),
        Err(_) => failures.push("A tool request stopped unexpectedly".into()),
    }
}

fn joined(failures: Vec<String>) -> Result<()> {
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failure(failures.join("\n")))
    }
}

#[cfg(test)]
mod tests;
