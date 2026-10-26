use super::{Reaper, prepare, read_capped, reaper::Readers, terminate};
use crate::error::{Error, Result, failure};
use std::fmt;
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

type Reader = JoinHandle<Result<String>>;

struct Capture {
    child: Option<Child>,
    stdout: Option<Reader>,
    stderr: Option<Reader>,
    reaper: Reaper,
}

pub(crate) fn capture(
    command: &mut Command,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<String> {
    let mut capture = Capture::start(command)?;
    let outcome = wait(capture.child()?, cancelled, timeout);
    capture.finish(outcome)
}

impl Capture {
    fn start(command: &mut Command) -> Result<Self> {
        let reaper = Reaper::start()?;
        prepare(command)?;
        let mut capture = Self {
            child: Some(command.spawn()?),
            stdout: None,
            stderr: None,
            reaper,
        };
        drop(capture.child()?.stdin.take());

        if let Err(error) = capture.start_readers() {
            // Startup owns the child before any fallible pipe/thread setup. Use the
            // same cleanup path as execution so even a partial start is reaped.
            return match capture.finish(Err(error)) {
                Err(error) => Err(error),
                Ok(_) => Err(failure("Process startup failed without an error")),
            };
        }
        Ok(capture)
    }

    fn child(&mut self) -> Result<&mut Child> {
        self.child
            .as_mut()
            .ok_or_else(|| failure("The command is closed"))
    }

    fn start_readers(&mut self) -> Result<()> {
        let stdout = self
            .child()?
            .stdout
            .take()
            .ok_or("Missing process output")?;
        self.stdout = Some(spawn_reader("prism-process-output", stdout, 1024 * 1024)?);

        let stderr = self
            .child()?
            .stderr
            .take()
            .ok_or("Missing process diagnostics")?;
        self.stderr = Some(spawn_reader(
            "prism-process-diagnostics",
            stderr,
            64 * 1024,
        )?);
        Ok(())
    }

    fn finish(mut self, outcome: Result<ExitStatus>) -> Result<String> {
        // Stop the whole process group before joining either reader. Descendants
        // can keep pipes open even after the command itself has exited.
        let mut child = self.child.take().ok_or("The command is closed")?;
        if let Err(error) = terminate(&mut child) {
            let readers = self.take_readers();
            self.reaper.retire(child, readers);
            return complete(outcome, Err(error), Ok(String::new()), Ok(String::new()));
        }
        let stdout = join_reader(self.stdout.take(), "output");
        let stderr = join_reader(self.stderr.take(), "diagnostic");

        complete(outcome, Ok(()), stdout, stderr)
    }

    fn take_readers(&mut self) -> Readers {
        Readers::Captured {
            stdout: self.stdout.take(),
            stderr: self.stderr.take(),
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            let readers = self.take_readers();
            self.reaper.retire(child, readers);
        }
    }
}

fn spawn_reader(
    name: &str,
    reader: impl std::io::Read + Send + 'static,
    limit: usize,
) -> Result<Reader> {
    Ok(std::thread::Builder::new()
        .name(name.into())
        .spawn(move || read_capped(reader, limit))?)
}

fn join_reader(reader: Option<Reader>, name: &str) -> Result<String> {
    let Some(reader) = reader else {
        // A failed startup may not have created this reader yet.
        return Ok(String::new());
    };
    reader
        .join()
        .map_err(|_| failure(format!("The {name} reader stopped")))?
}

fn complete(
    outcome: Result<ExitStatus>,
    cleanup: Result<()>,
    stdout: Result<String>,
    stderr: Result<String>,
) -> Result<String> {
    if outcome.is_err() || cleanup.is_err() || stdout.is_err() || stderr.is_err() {
        let failures = [
            outcome.err().map(|error| ("Command", error)),
            cleanup.err().map(|error| ("Process cleanup", error)),
            stdout.err().map(|error| ("Output", error)),
            stderr.err().map(|error| ("Diagnostics", error)),
        ]
        .into_iter()
        .flatten()
        .collect();
        return Err(Box::new(CaptureFailure { failures }));
    }

    let status = outcome?;
    let stdout = stdout?;
    let stderr = stderr?;
    if !status.success() {
        return Err(failure(format!(
            "Command failed ({status}):\n{stdout}\n{stderr}"
        )));
    }
    Ok(format!("{stdout}{stderr}"))
}

#[derive(Debug)]
struct CaptureFailure {
    failures: Vec<(&'static str, Error)>,
}

impl fmt::Display for CaptureFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, (phase, error)) in self.failures.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{phase}: {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CaptureFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.failures.first().map(|(_, error)| error.as_ref() as _)
    }
}

fn wait(child: &mut Child, cancelled: &AtomicBool, timeout: Duration) -> Result<ExitStatus> {
    let started = Instant::now();
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Cancelled"));
        }
        if started.elapsed() > timeout {
            return Err(failure("The command timed out"));
        }
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::capture;
    use crate::error::Result;
    use std::process::Command;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    #[test]
    fn capture_collects_both_streams_and_reports_unsuccessful_exit() -> Result<()> {
        let output = capture(
            Command::new("sh").args(["-c", "printf output; printf diagnostic >&2"]),
            &AtomicBool::new(false),
            Duration::from_secs(5),
        )?;
        assert_eq!(output, "outputdiagnostic");

        let error = capture(
            Command::new("sh").args(["-c", "printf failure >&2; exit 7"]),
            &AtomicBool::new(false),
            Duration::from_secs(5),
        )
        .expect_err("a command exiting with status 7 must fail");
        assert!(error.to_string().contains("failure"), "{error}");
        Ok(())
    }

    #[test]
    fn cancellation_and_timeout_stop_commands_with_open_output_pipes() {
        for (cancelled, timeout, message) in [
            (true, Duration::from_secs(5), "Cancelled"),
            (false, Duration::from_millis(10), "timed out"),
        ] {
            let error = capture(
                Command::new("sleep").arg("30"),
                &AtomicBool::new(cancelled),
                timeout,
            )
            .expect_err("interrupted commands must fail after closing their pipes");
            assert!(error.to_string().contains(message), "{error}");
            assert!(error.source().is_some(), "retain the interruption cause");
        }
    }
}
