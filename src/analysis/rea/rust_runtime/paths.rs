use super::super::launch;
use crate::error::{Result, failure};
use crate::settings::Settings;
use std::path::{Path, PathBuf};

pub(in super::super) fn launcher(settings: &Settings) -> Result<PathBuf> {
    if let Some(configured) = override_path("PRISM_RUST_REA")? {
        return launch::resolve(&configured);
    }
    if !settings.tools.rust_rea.is_empty() {
        return launch::resolve(&settings.tools.rust_rea);
    }

    for candidate in resource_launchers()? {
        if let Some(launcher) = rust_launcher(candidate)? {
            return Ok(launcher);
        }
    }
    for directory in crate::executable_paths::search_path() {
        if let Some(launcher) = rust_launcher(directory.join("rea"))? {
            return Ok(launcher);
        }
    }

    Err(failure(
        "Cannot find a Rust-enabled REA launcher. Set PRISM_RUST_REA or choose the cloned package's scripts/rea.mjs in Settings > Engines. Build it with `npm ci`, `npm run build:cached`, and `npm run build:ghidra:rust-worker`.",
    ))
}

pub(super) fn worker(settings: &Settings, launcher: &Path) -> Result<PathBuf> {
    if let Some(configured) = override_path("PRISM_RUST_GHIDRA_WORKER")? {
        return Ok(PathBuf::from(configured));
    }
    if !settings.tools.rust_ghidra_worker.is_empty() {
        return Ok(PathBuf::from(&settings.tools.rust_ghidra_worker));
    }

    let package = super::super::rust_package::package_root(launcher)?;
    let executable = if cfg!(windows) {
        "rea-ghidra-rust-worker.exe"
    } else {
        "rea-ghidra-rust-worker"
    };
    let bundled = package.join("bin").join(executable);
    if is_file(&bundled)? {
        return Ok(bundled);
    }

    Ok(package
        .join("native/ghidra-rust-worker/target/release")
        .join(executable))
}

fn override_path(name: &str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(failure(format!("Cannot read {name}: {error}"))),
    }
}

fn resource_launchers() -> Result<Vec<PathBuf>> {
    let executable = std::env::current_exe()?;
    let mut candidates = Vec::new();
    for parent in executable.ancestors().skip(1).take(4) {
        for relative in [
            "Resources/rea/scripts/rea.mjs",
            "resources/rea/scripts/rea.mjs",
        ] {
            candidates.push(parent.join(relative));
        }
    }
    Ok(candidates)
}

fn rust_launcher(candidate: PathBuf) -> Result<Option<PathBuf>> {
    if !is_file(&candidate)? {
        return Ok(None);
    }
    let launcher = candidate.canonicalize()?;
    let package = super::super::rust_package::package_root(&launcher)?;
    if is_file(&package.join("dist/ghidra-rust/GhidraRustProvider.js"))? {
        super::super::rust_package::validate(&launcher)?;
        return Ok(Some(launcher));
    }
    Ok(None)
}

fn is_file(path: &Path) -> Result<bool> {
    match path.metadata() {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(failure(format!(
            "Cannot inspect the Rust REA installation at {}: {error}",
            path.display(),
        ))),
    }
}
