use crate::error::{Result, failure};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(crate) fn prepare(command: &mut Command) {
    if let Ok(path) = std::env::join_paths(crate::executable_paths::search_path()) {
        command.env("PATH", path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
}

pub(crate) fn terminate(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    {
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
    }
    #[cfg(windows)]
    {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        let status = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            child.kill()?;
        }
    }
    child.wait()?;
    Ok(())
}

pub(crate) fn capture(
    command: &mut Command,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<String> {
    prepare(command);
    let mut child = command.spawn()?;
    drop(child.stdin.take());
    let output = child.stdout.take().ok_or("Missing process output")?;
    let stderr = child.stderr.take().ok_or("Missing process diagnostics")?;
    let stdout = std::thread::spawn(move || read_capped(output, 1024 * 1024));
    let errors = std::thread::spawn(move || read_capped(stderr, 64 * 1024));
    let outcome = wait(&mut child, cancelled, timeout);
    let cleanup = terminate(&mut child);
    let text = stdout
        .join()
        .map_err(|_| failure("The output reader stopped"))
        .and_then(|result| result);
    let diagnostics = errors
        .join()
        .map_err(|_| failure("The diagnostic reader stopped"))
        .and_then(|result| result);
    let mut problems = Vec::new();
    if let Err(error) = &outcome {
        problems.push(error.to_string());
    }
    if let Err(error) = cleanup {
        problems.push(format!("Process cleanup: {error}"));
    }
    if let Err(error) = &text {
        problems.push(format!("Output: {error}"));
    }
    if let Err(error) = &diagnostics {
        problems.push(format!("Diagnostics: {error}"));
    }
    if !problems.is_empty() {
        return Err(failure(problems.join("; ")));
    }
    let status = outcome?;
    let text = text?;
    let diagnostics = diagnostics?;
    if !status.success() {
        return Err(failure(format!(
            "Command failed ({status}):\n{text}\n{diagnostics}"
        )));
    }
    Ok(format!("{text}{diagnostics}"))
}

fn wait(
    child: &mut Child,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
    let started = Instant::now();
    loop {
        if cancelled.load(Ordering::Relaxed) || started.elapsed() > timeout {
            let original = if cancelled.load(Ordering::Relaxed) {
                "Cancelled"
            } else {
                "The command timed out"
            };
            return Err(failure(original));
        }
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                return Err(error.into());
            }
        }
    }
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
