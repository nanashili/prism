use super::catalog::Entry;
use crate::error::{Result, failure};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

fn root(entry: &Entry) -> Result<PathBuf> {
    if !super::catalog::valid_id(&entry.id)
        || entry.version.is_empty()
        || entry.version.len() > 100
        || entry.version.starts_with('.')
        || !entry
            .version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+'))
    {
        return Err(failure(
            "The agent registry contains an invalid identifier or version",
        ));
    }
    Ok(crate::settings::support_dir()
        .join("agents/bin")
        .join(&entry.id)
        .join(&entry.version))
}

pub(super) fn location(entry: &Entry, build: &Value) -> Result<PathBuf> {
    let command = Path::new(
        build["cmd"]
            .as_str()
            .ok_or("No agent build for this platform")?,
    );
    if command.is_absolute()
        || command
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(failure(
            "The agent executable escapes its install directory",
        ));
    }
    Ok(root(entry)?.join(command))
}

pub(super) fn binary(entry: &Entry, build: &Value, cancelled: &AtomicBool) -> Result<String> {
    let command = location(entry, build)?;
    if command.is_file() {
        return Ok(command.to_string_lossy().into_owned());
    }
    let destination = root(entry)?;
    let parent = destination
        .parent()
        .ok_or("The agent install directory has no parent")?;
    std::fs::create_dir_all(parent)?;
    let staging = tempfile::tempdir_in(parent)?;
    let archive = staging.path().join("download");
    super::download::cached(
        build["archive"]
            .as_str()
            .ok_or("The agent has no download URL")?,
        &archive,
        512 * 1024 * 1024,
        cancelled,
    )?;
    if let Some(expected) = build["sha256"].as_str() {
        let actual = crate::project::files::hash(&archive, cancelled)?;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(failure("The downloaded agent did not match its checksum"));
        }
    }
    let unpacked = staging.path().join("unpacked");
    std::fs::create_dir(&unpacked)?;
    let mut extraction = Command::new(crate::executable_paths::find("tar"));
    extraction
        .args(["-xf"])
        .arg(&archive)
        .arg("-C")
        .arg(&unpacked)
        .args(["--no-same-owner", "--no-same-permissions"]);
    crate::protocol::process::capture(&mut extraction, cancelled, Duration::from_secs(120))?;
    let relative = command.strip_prefix(&destination)?;
    let executable = std::fs::canonicalize(unpacked.join(relative))?;
    if !executable.starts_with(std::fs::canonicalize(&unpacked)?) || !executable.is_file() {
        return Err(failure("The downloaded agent has no valid executable"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&unpacked, &destination)?;
    Ok(command.to_string_lossy().into_owned())
}
