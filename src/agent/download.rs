use crate::error::{Result, failure};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub(super) fn cached(
    url: &str,
    destination: &Path,
    limit: u64,
    cancelled: &AtomicBool,
) -> Result<()> {
    if !url.starts_with("https://") {
        return Err(failure("Agent downloads require an HTTPS URL"));
    }
    let parent = destination
        .parent()
        .ok_or("The download has no parent directory")?;
    std::fs::create_dir_all(parent)?;
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    let mut command = Command::new(crate::executable_paths::find("curl"));
    command
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "5",
            "--max-time",
            "60",
            "--max-filesize",
        ])
        .arg(limit.to_string())
        .arg("--output")
        .arg(temporary.path())
        .arg("--url")
        .arg(url);
    crate::protocol::process::capture(&mut command, cancelled, Duration::from_secs(65))?;
    if temporary.as_file().metadata()?.len() > limit {
        return Err(failure("The agent download exceeds its size limit"));
    }
    temporary.as_file().sync_all()?;
    temporary.persist(destination)?;
    Ok(())
}
