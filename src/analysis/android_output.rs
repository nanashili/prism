use crate::error::{Result, failure};
use crate::model::Target;
use crate::settings::{Settings, support_dir};
use crate::storage::artifacts;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub(super) fn cache(target: &Target) -> PathBuf {
    support_dir().join("android").join(&target.sha256)
}

pub(super) fn decompile(
    target: &Target,
    settings: &Settings,
    cancelled: &AtomicBool,
) -> Result<PathBuf> {
    let root = cache(target);
    let budget = settings.artifact_cache_mib * 1024 * 1024;
    if root.join("sources/mapping.json").is_file() {
        artifacts::reserve(0, budget, &[&target.path, &root], cancelled)?;
        artifacts::touch(&root)?;
        return Ok(root);
    }
    let parent = root.parent().ok_or("The Android cache has no parent")?;
    std::fs::create_dir_all(parent)?;
    let temporary = tempfile::tempdir_in(parent)?;
    let prepared = generate(temporary.path(), target, settings, cancelled)
        .and_then(|()| validate(temporary.path(), &target.path, budget, cancelled))
        .and_then(|()| artifacts::touch(temporary.path()));
    if let Err(original) = prepared {
        return match temporary.close() {
            Ok(()) => Err(original),
            Err(cleanup) => Err(failure(format!(
                "{original}; removing partial Android output failed: {cleanup}"
            ))),
        };
    }

    let completed = temporary.keep();
    std::fs::rename(&completed, &root).map_err(|error| {
        failure(format!(
            "{error}; decompiled output remains at {}",
            completed.display()
        ))
    })?;
    Ok(root)
}

fn generate(
    output: &Path,
    target: &Target,
    settings: &Settings,
    cancelled: &AtomicBool,
) -> Result<()> {
    let mut command = Command::new(&settings.jadx);
    command
        .args([
            "--output-format",
            "json",
            "--threads-count",
            "1",
            "--log-level",
            "error",
            "-d",
        ])
        .arg(output)
        .arg(&target.path)
        .env(
            "JAVA_OPTS",
            format!("-Xmx{}m", settings.analysis_memory_mib),
        );
    crate::protocol::process::capture(
        &mut command,
        cancelled,
        Duration::from_secs(settings.timeout_seconds),
    )?;
    Ok(())
}

fn validate(output: &Path, input: &Path, budget: u64, cancelled: &AtomicBool) -> Result<()> {
    if !output.join("sources/mapping.json").is_file() {
        return Err(failure("jadx completed without a class mapping"));
    }
    let bytes = artifacts::directory_bytes(output, &mut 0, cancelled)?;
    artifacts::reserve(bytes, budget, &[input], cancelled)
}
