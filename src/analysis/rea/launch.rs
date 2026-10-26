use crate::error::Result;
use crate::settings::{AnalysisBackend, Settings};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

pub(super) struct Launch {
    pub command: Command,
    pub provider: String,
    pub timeout: Duration,
}

impl Launch {
    pub fn new(settings: &Settings) -> Result<Self> {
        let backend = settings.analysis_backend();
        backend.require_available()?;
        let command = configured_command(settings, backend)?;
        let provider = backend.rea_provider(&settings.provider);
        let timeout = Duration::from_secs(settings.timeout_seconds);
        Ok(Self {
            command,
            provider: provider.into(),
            timeout,
        })
    }
}

fn configured_command(settings: &Settings, backend: AnalysisBackend) -> Result<Command> {
    let rea = rea_executable(settings)?;
    let mut command = Command::new(&rea);
    command.arg("mcp");
    command.env(
        "REA_ANALYSIS_PROVIDER",
        backend.rea_provider(&settings.provider),
    );
    command.env("REA_RIZIN_COMMAND", &settings.rizin);
    if backend == AnalysisBackend::RustGhidraRea {
        super::rust_runtime::configure(&mut command, settings, &rea)?;
    }

    Ok(command)
}

pub(super) fn rea_executable(settings: &Settings) -> Result<PathBuf> {
    super::rust_runtime::launcher(settings)
}
