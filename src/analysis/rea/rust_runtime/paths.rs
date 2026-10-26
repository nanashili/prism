use crate::error::{Result, failure};
use crate::settings::Settings;
use std::path::{Path, PathBuf};

pub(in super::super) fn launcher(settings: &Settings) -> Result<PathBuf> {
    if let Some(configured) = override_path("PRISM_RUST_REA")? {
        return native_launcher(PathBuf::from(configured), true)?
            .ok_or_else(|| failure("PRISM_RUST_REA must point to a native REA executable"));
    }
    if let Some(configured) = override_path("PRISM_REA")? {
        return native_launcher(PathBuf::from(configured), true)?
            .ok_or_else(|| failure("PRISM_REA must point to a native REA executable"));
    }
    if !settings.rea.is_empty() && settings.rea != "rea" {
        return native_launcher(PathBuf::from(&settings.rea), true)?
            .ok_or_else(|| failure("The configured REA path must point to a native executable"));
    }

    for candidate in bundled_launchers()? {
        if let Some(launcher) = native_launcher(candidate, false)? {
            return Ok(launcher);
        }
    }

    let executable = if cfg!(windows) { "rea.exe" } else { "rea" };
    for directory in crate::executable_paths::search_path() {
        if let Some(launcher) = native_launcher(directory.join(executable), false)? {
            return Ok(launcher);
        }
    }

    Err(failure(
        "Cannot find the native REA executable. Download nanashili/rea-rust during setup, set PRISM_RUST_REA, or choose its `rea` executable in Settings > Engines.",
    ))
}

pub(super) fn worker(settings: &Settings, launcher: &Path) -> Result<PathBuf> {
    if let Some(configured) = override_path("PRISM_RUST_GHIDRA_WORKER")? {
        return Ok(PathBuf::from(configured));
    }
    if !settings.tools.rust_ghidra_worker.is_empty() {
        return Ok(PathBuf::from(&settings.tools.rust_ghidra_worker));
    }

    let executable = if cfg!(windows) {
        "rea-ghidra-rust-worker.exe"
    } else {
        "rea-ghidra-rust-worker"
    };
    let launcher_directory = launcher
        .parent()
        .ok_or_else(|| failure("The configured native REA executable has no parent directory"))?;
    let candidates = [
        launcher_directory.join(executable),
        launcher_directory.join("bin").join(executable),
        launcher_directory
            .join("../ghidra-rust-worker/target/release")
            .join(executable),
    ];
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            failure(format!(
                "Cannot find the native REA Ghidra worker beside {}. Set PRISM_RUST_GHIDRA_WORKER to its absolute path.",
                launcher.display()
            ))
        })
}

fn override_path(name: &str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(failure(format!("Cannot read {name}: {error}"))),
    }
}

fn bundled_launchers() -> Result<Vec<PathBuf>> {
    let executable = std::env::current_exe()?;
    let mut candidates = Vec::new();
    for parent in executable.ancestors().skip(1).take(4) {
        for relative in ["Resources/rea-rust/bin", "resources/rea-rust/bin"] {
            let executable = if cfg!(windows) { "rea.exe" } else { "rea" };
            candidates.push(parent.join(relative).join(executable));
        }
    }
    Ok(candidates)
}

fn native_launcher(candidate: PathBuf, explicit: bool) -> Result<Option<PathBuf>> {
    if !is_file(&candidate)? {
        return Ok(None);
    }
    let launcher = candidate.canonicalize()?;
    if !explicit && !super::super::rust_package::has_native_header(&launcher)? {
        return Ok(None);
    }
    if !explicit && !is_executable_file(&launcher)? {
        return Ok(None);
    }
    super::super::rust_package::validate(&launcher)?;
    Ok(Some(launcher))
}

fn is_executable_file(path: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Ok(path.metadata()?.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        Ok(true)
    }
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
