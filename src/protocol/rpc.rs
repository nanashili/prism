use super::{Framing, process};
use crate::error::{Result, failure};
use serde_json::{Value, json};
use std::io::BufReader;
use std::process::{ChildStdin, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

pub(crate) struct Rpc {
    process: process::Managed,
    input: ChildStdin,
    incoming: Option<mpsc::Receiver<Result<Value>>>,
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
        let mut process = process::Managed::spawn(command)?;
        let child = process.child()?;
        let input = child.stdin.take().ok_or("Missing server input")?;
        let output = child.stdout.take().ok_or("Missing server output")?;
        let diagnostics = child.stderr.take().ok_or("Missing server diagnostics")?;
        let (sender, incoming) = mpsc::sync_channel(16);
        let reader = std::thread::Builder::new()
            .name("prism-rpc-output".into())
            .spawn(move || {
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
            })?;
        process.track_reader(reader);
        let errors = std::thread::Builder::new()
            .name("prism-rpc-diagnostics".into())
            .spawn(move || match process::read_capped(diagnostics, 16 * 1024) {
                Ok(text) if !text.trim().is_empty() => eprintln!("server: {text}"),
                Err(error) => eprintln!("Cannot read server diagnostics: {error}"),
                _ => {}
            })?;
        process.track_reader(errors);
        Ok(Self {
            process,
            input,
            incoming: Some(incoming),
            framing,
            next: 0,
            memory_limit: None,
            synchronized: true,
            cancelled,
            replies: Default::default(),
        })
    }

    pub fn close(&mut self) -> Result<()> {
        self.incoming.take();
        self.process.close()
    }

    pub fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.send(&json!({"jsonrpc":"2.0","method":method,"params":params}))
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
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

    #[cfg_attr(feature = "profiling", hotpath::measure)]
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
        self.next = self
            .next
            .checked_add(1)
            .ok_or("The server request ID space is exhausted")?;
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
            && let Some(id) = self.process.id()
            && let Some(bytes) = super::memory::resident(id)?
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

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn poll(&self, timeout: Duration) -> Result<Option<Value>> {
        let incoming = self.incoming.as_ref().ok_or("The server is closed")?;
        match incoming.recv_timeout(timeout) {
            Ok(message) => Ok(Some(message?)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(failure("The server process closed its output"))
            }
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl Drop for Rpc {
    fn drop(&mut self) {
        // Closing the receiver releases a reader waiting to deliver its last
        // response; Managed transfers process waiting to the cleanup worker.
        self.incoming.take();
    }
}
