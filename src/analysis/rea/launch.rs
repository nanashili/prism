use crate::error::{Result, failure};
use crate::settings::{AnalysisBackend, Settings};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub(super) struct Launch {
    pub command: Command,
    pub provider: String,
    pub timeout: Duration,
    pub opening_timeout: Duration,
}

impl Launch {
    pub fn new(settings: &Settings) -> Result<Self> {
        let backend = settings.analysis_backend();
        backend.require_available()?;
        let command = configured_command(settings, backend)?;
        let provider = backend.rea_provider(&settings.provider);
        let timeout = Duration::from_secs(settings.timeout_seconds);
        let opening_timeout = if backend == AnalysisBackend::JavaGhidraRea {
            super::java_runtime::startup_timeout(settings)?.saturating_add(Duration::from_secs(60))
        } else {
            timeout
        };
        Ok(Self {
            command,
            provider: provider.into(),
            timeout: if backend.retains_prepared_session() {
                timeout.max(opening_timeout)
            } else {
                timeout
            },
            opening_timeout,
        })
    }
}

fn configured_command(settings: &Settings, backend: AnalysisBackend) -> Result<Command> {
    let rea = rea_executable(settings)?;
    let node = script_node(settings, &rea)?;
    let mut command = match &node {
        Some(node) => {
            let mut command = Command::new(node);
            command.arg(&rea);
            command
        }
        None => Command::new(&rea),
    };
    command.arg("mcp");
    command.env(
        "REA_ANALYSIS_PROVIDER",
        backend.rea_provider(&settings.provider),
    );
    command.env("REA_RIZIN_COMMAND", &settings.rizin);
    if backend == AnalysisBackend::RustGhidraRea {
        super::rust_package::validate(&rea)?;
        super::rust_runtime::configure(&mut command, settings, &rea)?;
    } else {
        super::java_runtime::configure_ghidra(
            &mut command,
            settings,
            backend == AnalysisBackend::JavaGhidraRea,
        )?;
    }

    Ok(command)
}

pub(super) fn resolve(configured: &str) -> Result<PathBuf> {
    let path = Path::new(configured);
    let found = if path.components().count() > 1 || path.is_absolute() {
        path.to_owned()
    } else {
        PathBuf::from(crate::executable_paths::find(configured))
    };
    if configured.is_empty() || !found.is_file() {
        return Err(failure(format!(
            "Cannot find the configured executable {configured:?}. Choose its path in Settings > Engines.",
        )));
    }
    Ok(found.canonicalize()?)
}

pub(super) fn script_node(settings: &Settings, rea: &Path) -> Result<Option<PathBuf>> {
    if !matches!(
        rea.extension().and_then(|value| value.to_str()),
        Some("js" | "mjs")
    ) {
        return Ok(None);
    }
    if !settings.tools.rea_node.is_empty() {
        return resolve(&settings.tools.rea_node).map(Some);
    }
    let bundled = rea
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(|resources| resources.join("node").join("bin").join("node"));
    if let Some(path) = bundled.filter(|path| path.is_file()) {
        return Ok(Some(path.canonicalize()?));
    }
    resolve("node").map(Some)
}

pub(super) fn rea_executable(settings: &Settings) -> Result<PathBuf> {
    if settings.analysis_backend() == AnalysisBackend::RustGhidraRea {
        super::rust_runtime::launcher(settings)
    } else {
        resolve(&settings.rea)
    }
}
