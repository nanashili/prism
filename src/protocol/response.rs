use super::Rpc;
use crate::error::{Result, failure};
use serde_json::{Value, json};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

impl Rpc {
    pub fn reply(&mut self, id: &Value, result: Value) -> Result<()> {
        self.send(&json!({"jsonrpc":"2.0","id":id,"result":result}))
    }

    /// Complete an exchange after its caller has matched the response identifier.
    pub fn finish_call(&mut self, message: Value) -> Result<Value> {
        self.synchronized = true;
        response_result(message)
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn dispatch(
        &mut self,
        message: Value,
        mut event: impl FnMut(&Value) -> Result<Option<Value>>,
    ) -> Result<()> {
        if let Some(id) = response_id(&message) {
            if self.replies.len() >= 16 {
                return Err(failure("Too many outstanding server replies"));
            }
            self.replies.insert(id, message);
            return Ok(());
        }

        let response = event(&message)?;
        if message.get("method").is_some()
            && let Some(id) = message.get("id")
        {
            let reply = match response {
                Some(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
                None => json!({"jsonrpc":"2.0","id":id,"error":{
                    "code":-32601,"message":"Unsupported client request"
                }}),
            };
            self.send(&reply)?;
        }
        Ok(())
    }

    pub fn take_reply(&mut self, id: u64) -> Option<Value> {
        self.replies.remove(&id)
    }

    pub fn wait(
        &mut self,
        id: u64,
        method: &str,
        timeout: Duration,
        event: impl FnMut(&Value) -> Result<Option<Value>>,
    ) -> Result<Value> {
        self.wait_with_control(id, method, timeout, event, |_| Ok(()))
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn wait_with_control(
        &mut self,
        id: u64,
        method: &str,
        timeout: Duration,
        mut event: impl FnMut(&Value) -> Result<Option<Value>>,
        mut control: impl FnMut(&mut Self) -> Result<()>,
    ) -> Result<Value> {
        let started = Instant::now();
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            if started.elapsed() > timeout {
                return Err(failure(format!("{method} timed out")));
            }
            control(self)?;
            self.check_memory()?;
            let message = match self.replies.remove(&id) {
                Some(message) => Some(message),
                None => self.poll(Duration::from_millis(100))?,
            };
            let Some(mut message) = message else {
                continue;
            };
            if message.get("id").and_then(Value::as_u64) == Some(id)
                && message.get("method").is_none()
            {
                return self.finish_call(message);
            }
            if message.get("request_seq").and_then(Value::as_u64) == Some(id) {
                self.synchronized = true;
                if message.get("success").and_then(Value::as_bool) == Some(false) {
                    return Err(failure(
                        message["message"]
                            .as_str()
                            .unwrap_or("Debugger request failed"),
                    ));
                }
                return Ok(message.get_mut("body").map_or(Value::Null, Value::take));
            }
            self.dispatch(message, &mut event)?;
        }
    }
}

pub(crate) fn response_id(message: &Value) -> Option<u64> {
    if message.get("method").is_some() {
        return None;
    }
    message
        .get("request_seq")
        .or_else(|| message.get("id"))
        .and_then(Value::as_u64)
}

pub(crate) fn response_result(mut message: Value) -> Result<Value> {
    if let Some(error) = message.get("error") {
        return Err(failure(error.to_string()));
    }
    Ok(message.get_mut("result").map_or(Value::Null, Value::take))
}
