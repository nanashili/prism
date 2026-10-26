use crate::error::Result;
use std::process::{Child, Command, Stdio};

mod capture;
pub(crate) use capture::capture;
mod reaper;
use reaper::Reaper;
mod managed;
pub(crate) use managed::Managed;

pub(crate) fn prepare(command: &mut Command) -> Result<()> {
    let path = std::env::join_paths(crate::executable_paths::search_path())?;
    command.env("PATH", path);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(())
}

pub(crate) fn terminate(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    signal(child)?;
    #[cfg(windows)]
    {
        if child.try_wait()?.is_none() {
            let status = Command::new("taskkill")
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if !status.success() {
                child.kill()?;
            }
        }
    }
    child.wait()?;
    Ok(())
}

/// Request termination without waiting for the process or its output readers.
#[cfg(unix)]
pub(crate) fn signal(child: &mut Child) -> Result<()> {
    let group = i32::try_from(child.id())?;
    // SAFETY: kill accepts this integer process-group ID and signal without any pointers.
    // The child was started as a new group; the negative pid addresses only that group.
    let result = unsafe { libc::kill(-group, libc::SIGKILL) };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error.into());
        }
    }
    Ok(())
}

pub(super) fn read_capped(mut reader: impl std::io::Read, limit: usize) -> Result<String> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = limit.saturating_sub(output.len());
        output.extend_from_slice(&buffer[..count.min(remaining)]);
        truncated |= count > remaining;
    }
    let mut text = String::from_utf8_lossy(&output).into_owned();
    if truncated {
        text.push_str("\n[Output truncated to its display limit]\n");
    }
    Ok(text)
}
