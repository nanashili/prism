use crate::error::{Result, failure};
use crate::events::{Event, Sink};
use crate::protocol::Rpc;
use crate::settings::{AnalysisBackend, Settings};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

#[cfg(feature = "rea-rust-ghidra")]
mod batch;
#[cfg(feature = "rea-rust-ghidra")]
mod batch_item;
#[cfg(feature = "rea-rust-ghidra")]
mod batch_parse;
#[cfg(feature = "rea-rust-ghidra")]
mod batch_state;
#[cfg(feature = "rea-rust-ghidra")]
mod batch_transport;
mod connection;
#[cfg(feature = "rea-rust-ghidra")]
mod error_projection;
#[cfg(feature = "rea-rust-ghidra")]
mod identity;
mod launch;
mod reply;
mod rust_package;
mod rust_runtime;
mod shutdown;

#[cfg(feature = "rea-rust-ghidra")]
pub(crate) use batch_state::{DecompilationBatch, DecompilationBatchPoll, DecompilationItem};
#[cfg(feature = "rea-rust-ghidra")]
pub(crate) use identity::backend_identity;

pub(crate) struct Rea {
    rpc: Rpc,
    timeout: Duration,
    target: Option<String>,
    provider: String,
    backend: AnalysisBackend,
    sink: Sink,
    #[cfg(feature = "rea-rust-ghidra")]
    session: Arc<()>,
    #[cfg(feature = "rea-rust-ghidra")]
    active_batch: Option<u64>,
    #[cfg(feature = "rea-rust-ghidra")]
    abandoned: Option<batch_state::PendingCall>,
    healthy: bool,
}

impl Rea {
    /// Check that the selected native REA executable and its Ghidra worker
    /// can be launched before setup commits the engine choice.
    pub(crate) fn validate_installation(settings: &Settings) -> Result<()> {
        let backend = settings.analysis_backend();
        backend.require_available()?;
        let rea = launch::rea_executable(settings)?;
        if backend == AnalysisBackend::RustGhidraRea {
            rust_package::validate(&rea)?;
            rust_runtime::worker_executable(settings, &rea)?;
        }
        Ok(())
    }

    pub fn is_reusable(&self) -> bool {
        #[cfg(feature = "rea-rust-ghidra")]
        let pending = self.active_batch.is_some() || self.abandoned.is_some();
        #[cfg(not(feature = "rea-rust-ghidra"))]
        let pending = false;
        self.healthy && (self.rpc.is_reusable() || pending)
    }

    pub fn retains_prepared_session(&self) -> bool {
        self.backend.retains_prepared_session() && self.target.is_some() && self.healthy
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn connect(settings: &Settings, cancelled: Arc<AtomicBool>, sink: Sink) -> Result<Self> {
        let mut launch = launch::Launch::new(settings)?;
        let backend = settings.analysis_backend();
        let rpc = connection::connect(&mut launch.command, backend, cancelled)?;

        Ok(Self {
            rpc,
            timeout: launch.timeout,
            target: None,
            provider: launch.provider,
            backend,
            sink,
            #[cfg(feature = "rea-rust-ghidra")]
            session: Arc::new(()),
            #[cfg(feature = "rea-rust-ghidra")]
            active_batch: None,
            #[cfg(feature = "rea-rust-ghidra")]
            abandoned: None,
            healthy: true,
        })
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn open(&mut self, path: &Path) -> Result<Value> {
        let text = path.to_string_lossy();
        if self.target.as_deref() == Some(&text) {
            return Ok(Value::Null);
        }
        self.target = None;
        let reply = self.call_tool(
            "open_binary",
            json!({"path":text,"provider_id":self.provider}),
            self.timeout,
        )?;
        let value = reply.into_result()?;
        if self.backend.retains_prepared_session() {
            self.sink.send(Event::Progress(format!(
                "Preparing {} analysis",
                self.backend.label(),
            )))?;
            self.call_tool("list_segments", json!({}), self.timeout)?
                .into_result()?;
        }
        self.target = Some(text.into_owned());
        Ok(value)
    }

    pub fn tool(&mut self, name: &str, args: Value) -> Result<Value> {
        if self.backend.retains_prepared_session() && name == "batch_decompile" {
            return Err(failure(
                "Use decompile_batch with action: start for prepared Ghidra bulk decompilation",
            ));
        }
        let reply = self.call_tool(name, args, self.timeout)?;
        if name == "list_procedures"
            && let reply::ToolReply::Failure(error) = &reply
            && let Some(evidence) = error.retained_evidence()
        {
            return Ok(
                json!({"delivery":"retained-inventory","evidence_reference":{"evidence_id":evidence}}),
            );
        }
        reply.into_result()
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    fn call_tool(
        &mut self,
        name: &str,
        args: Value,
        timeout: Duration,
    ) -> Result<reply::ToolReply> {
        #[cfg(feature = "rea-rust-ghidra")]
        self.ensure_available()?;
        let sink = self.sink.clone();
        let reply = self.rpc.call(
            "tools/call",
            json!({"name":name,"arguments":args}),
            timeout,
            |message| progress(&sink, message),
        )?;
        self.parse_tool_reply(reply)
    }

    fn parse_tool_reply(&mut self, value: Value) -> Result<reply::ToolReply> {
        let reply = match reply::parse(value) {
            Ok(reply) => reply,
            Err(error) => {
                if self.backend.retains_prepared_session() {
                    self.healthy = false;
                }
                return Err(error);
            }
        };
        if self.backend.retains_prepared_session()
            && let reply::ToolReply::Failure(error) = &reply
            && error.session_unavailable()
        {
            self.healthy = false;
        }
        Ok(reply)
    }

    pub fn tools(&mut self) -> Result<Value> {
        #[cfg(feature = "rea-rust-ghidra")]
        self.ensure_available()?;
        self.rpc
            .call("tools/list", json!({}), self.timeout, |_| Ok(None))
    }
}

fn progress(sink: &Sink, message: &Value) -> Result<Option<Value>> {
    if let Some(text) = message.pointer("/params/message").and_then(Value::as_str) {
        sink.send(Event::Progress(text.chars().take(240).collect()))?;
    }
    Ok(None)
}
