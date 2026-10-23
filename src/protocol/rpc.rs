use super::{Framing, process};
use crate::error::{Result, failure};
use serde_json::{Value, json};
use std::io::BufReader;
use std::process::{Child, ChildStdin, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(crate) struct Rpc {
    child: Child,
    input: ChildStdin,
    incoming: Option<super::incoming::Receiver>,
    readers: Vec<JoinHandle<()>>,
    framing: Framing,
    next: u64,
    memory_limit: Option<u64>,
    pub(super) synchronized: bool,
    pub(super) cancelled: Arc<AtomicBool>,
    pub(super) replies: std::collections::BTreeMap<u64, Value>,
}

impl Rpc {
    pub fn spawn(
        command: &mut Command,
        framing: Framing,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Self> {
        process::prepare(command);
        let mut child = command.spawn()?;
        let input = child.stdin.take().ok_or("Missing server input")?;
        let output = child.stdout.take().ok_or("Missing server output")?;
        let diagnostics = child.stderr.take().ok_or("Missing server diagnostics")?;
        let (sender, incoming) = super::incoming::channel();
        let reader = std::thread::spawn(move || {
            let mut output = BufReader::new(output);
            loop {
                let message = match framing.read(&mut output) {
                    Ok(Some(value)) => Ok(value),
                    Ok(None) => break,
                    Err(error) => Err(error),
                };
                let failed = message.is_err();
                if sender.send(message).is_err() || failed {
                    break;
                }
            }
        });
        let errors =
            std::thread::spawn(move || match process::read_capped(diagnostics, 16 * 1024) {
                Ok(text) if !text.trim().is_empty() => eprintln!("server: {text}"),
                Err(error) => eprintln!("Cannot read server diagnostics: {error}"),
                _ => {}
            });
        Ok(Self {
            child,
            input,
            incoming: Some(incoming),
            readers: vec![reader, errors],
            framing,
            next: 0,
            memory_limit: None,
            synchronized: true,
            cancelled,
            replies: Default::default(),
        })
    }

    pub fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.send(&json!({"jsonrpc":"2.0","method":method,"params":params}))
    }

    pub fn send(&mut self, message: &Value) -> Result<()> {
        let result = self.framing.write(&mut self.input, message);
        if result.is_err() {
            self.synchronized = false;
        }
        result
    }

    pub fn call(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
        event: impl FnMut(&Value) -> Result<Option<Value>>,
    ) -> Result<Value> {
        let id = self.begin_call(method, params)?;
        self.wait(id, method, timeout, event)
    }

    pub fn begin_call(&mut self, method: &str, params: Value) -> Result<u64> {
        if !self.synchronized {
            return Err(failure("The server connection needs to be reopened"));
        }
        self.synchronized = false;
        self.begin(method, params)
    }

    /// A completed error reply is reusable; an interrupted exchange is not.
    pub fn is_reusable(&self) -> bool {
        self.synchronized
    }

    pub fn begin(&mut self, method: &str, params: Value) -> Result<u64> {
        self.next += 1;
        let id = self.next;
        let payload = match self.framing {
            Framing::Zero => return Err(failure("Use the native command exchange for Rizin")),
            Framing::Dap => json!({"seq":id,"type":"request","command":method,"arguments":params}),
            _ => json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}),
        };
        self.send(&payload)?;
        Ok(id)
    }

    pub fn limit_memory(&mut self, bytes: u64) {
        self.memory_limit = Some(bytes);
    }

    pub(super) fn check_memory(&self) -> Result<()> {
        if let Some(limit) = self.memory_limit
            && let Some(bytes) = super::memory::resident(self.child.id())?
            && bytes > limit
        {
            return Err(failure(format!(
                "Analysis exceeded the {} MiB process memory budget. Raise the budget in Settings or inspect a smaller target.",
                limit / (1024 * 1024)
            )));
        }
        Ok(())
    }

    pub fn exchange(&mut self, command: Option<&str>, timeout: Duration) -> Result<Value> {
        if !self.synchronized {
            return Err(failure("The server connection needs to be reopened"));
        }
        self.synchronized = false;
        if let Some(command) = command {
            self.send(&Value::String(command.into()))?;
        }
        let started = Instant::now();
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            if started.elapsed() > timeout {
                return Err(failure("The analysis command timed out"));
            }
            self.check_memory()?;
            if let Some(value) = self.poll(Duration::from_millis(100))? {
                self.synchronized = true;
                return Ok(value);
            }
        }
    }

    pub fn poll(&self, timeout: Duration) -> Result<Option<Value>> {
        let incoming = self.incoming.as_ref().ok_or("The server is closed")?;
        incoming.poll(timeout)
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl Drop for Rpc {
    fn drop(&mut self) {
        self.incoming.take();
        if let Err(error) = process::terminate(&mut self.child) {
            eprintln!("Failed to stop server process {}: {error}", self.child.id());
        }
        for reader in self.readers.drain(..) {
            if reader.join().is_err() {
                eprintln!("A server output reader failed during shutdown");
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests;
