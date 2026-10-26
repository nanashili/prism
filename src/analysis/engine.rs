use super::{Rea, rizin::Rizin};
use crate::error::Result;
use crate::events::Sink;
use crate::settings::{AnalysisBackend, Settings};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::{Duration, Instant};

pub(crate) struct Engine {
    backend: Backend,
    last_used: Instant,
}

const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
mod batch;

enum Backend {
    Rea(Rea),
    Rizin(Rizin),
}

impl Engine {
    pub fn close(self) -> Result<()> {
        match self.backend {
            Backend::Rea(engine) => engine.close(),
            Backend::Rizin(engine) => engine.close(),
        }
    }

    pub fn open(
        settings: &Settings,
        path: &Path,
        cancelled: Arc<AtomicBool>,
        sink: Sink,
    ) -> Result<Self> {
        let selection = settings.analysis_backend();
        selection.require_available()?;
        let backend = match selection {
            AnalysisBackend::Rizin => Backend::Rizin(Rizin::open(settings, path, cancelled)?),
            _ => {
                let mut engine = Rea::connect(settings, cancelled, sink)?;
                engine.open(path)?;
                Backend::Rea(engine)
            }
        };
        Ok(Self {
            backend,
            last_used: Instant::now(),
        })
    }

    pub fn idle_remaining(&self) -> Duration {
        if matches!(&self.backend, Backend::Rea(engine) if engine.retains_prepared_session()) {
            return Duration::MAX;
        }
        IDLE_TIMEOUT.saturating_sub(self.last_used.elapsed())
    }

    pub fn is_reusable(&self) -> bool {
        match &self.backend {
            Backend::Rea(engine) => engine.is_reusable(),
            Backend::Rizin(engine) => engine.is_reusable(),
        }
    }

    pub fn index(&mut self, store: &mut crate::storage::Store, project: &str) -> Result<usize> {
        let result = match &mut self.backend {
            Backend::Rea(engine) => engine.index(store, project),
            Backend::Rizin(engine) => engine.index(store, project),
        };
        self.last_used = Instant::now();
        result
    }

    pub fn tool(&mut self, name: &str, args: Value) -> Result<Value> {
        let result = match &mut self.backend {
            Backend::Rea(engine) => engine.tool(name, args),
            Backend::Rizin(engine) => engine.tool(name, args),
        };
        self.last_used = Instant::now();
        result
    }

    pub fn normalized(&mut self, name: &str, args: Value) -> Result<Value> {
        let mut reply = self.tool(name, args)?;
        match reply.get_mut("normalized_result") {
            Some(result) => Ok(result.take()),
            None => Ok(reply),
        }
    }

    pub fn tools(&mut self) -> Result<Value> {
        match &mut self.backend {
            Backend::Rea(engine) => {
                let result = engine.tools();
                self.last_used = Instant::now();
                result
            }
            Backend::Rizin(_) => Ok(super::rizin_tools::definitions()),
        }
    }
}
