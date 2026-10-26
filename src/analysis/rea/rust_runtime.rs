use crate::error::{Result, failure};
use crate::settings::{Settings, support_dir};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

mod paths;

pub(super) use paths::launcher;

pub(super) fn worker_executable(settings: &Settings, launcher: &Path) -> Result<PathBuf> {
    let configured = paths::worker(settings, launcher)?;
    if !configured.is_absolute() {
        return Err(failure(
            "Set PRISM_RUST_GHIDRA_WORKER or choose an absolute REA Rust Ghidra worker path in Settings > Engines",
        ));
    }

    let worker = configured.canonicalize().map_err(|error| {
        failure(format!(
            "Cannot read REA's native Rust Ghidra worker at {}: {error}. Download the matching nanashili/rea-rust release with its worker, or choose the worker executable in Settings > Engines.",
            configured.display(),
        ))
    })?;
    super::rust_package::validate(&worker).map_err(|error| {
        failure(format!(
            "The configured REA Rust Ghidra worker is invalid at {}: {error}",
            worker.display()
        ))
    })?;

    Ok(worker)
}

pub(super) fn cache_dir() -> Result<PathBuf> {
    let directory = support_dir().join("ghidra-rust-rea");
    if !directory.is_absolute() {
        return Err(failure(
            "REA's Rust Ghidra cache requires an absolute Prism support directory",
        ));
    }
    Ok(directory)
}

pub(super) fn configure(command: &mut Command, settings: &Settings, launcher: &Path) -> Result<()> {
    let worker = worker_executable(settings, launcher)?;
    let cache = cache_dir()?;

    command.env("REA_GHIDRA_RUST_ENABLED", "true");
    command.env("REA_GHIDRA_RUST_WORKER", worker);
    command.env(
        "REA_GHIDRA_RUST_DECOMPILE_THREADS",
        settings.tools.ghidra_threads.to_string(),
    );
    command.env(
        "REA_GHIDRA_RUST_MEMORY_MIB",
        settings.analysis_memory_mib.to_string(),
    );
    command.env(
        "REA_GHIDRA_RUST_TIMEOUT_MS",
        Duration::from_secs(settings.timeout_seconds)
            .as_millis()
            .to_string(),
    );
    command.env("REA_GHIDRA_RUST_CACHE_DIR", cache);
    command.env("REA_MCP_MAX_RESPONSE_BYTES", (10 * 1024 * 1024).to_string());
    Ok(())
}
