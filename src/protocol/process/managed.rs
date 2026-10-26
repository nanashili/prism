use super::{Reaper, prepare, reaper::Readers, terminate};
use crate::error::{Result, failure, finish};
use std::process::{Child, Command};
use std::thread::JoinHandle;

/// Owns a helper from the moment it is spawned, including partial pipe setup.
pub(crate) struct Managed {
    child: Option<Child>,
    readers: Vec<JoinHandle<()>>,
    reaper: Reaper,
}

impl Managed {
    pub(crate) fn spawn(command: &mut Command) -> Result<Self> {
        let reaper = Reaper::start()?;
        prepare(command)?;
        Ok(Self {
            child: Some(command.spawn()?),
            readers: Vec::new(),
            reaper,
        })
    }

    pub(crate) fn child(&mut self) -> Result<&mut Child> {
        self.child
            .as_mut()
            .ok_or_else(|| failure("The server is closed"))
    }

    pub(crate) fn id(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }

    pub(crate) fn track_reader(&mut self, reader: JoinHandle<()>) {
        self.readers.push(reader);
    }

    pub(crate) fn close(&mut self) -> Result<()> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        let stopped = terminate(&mut child);
        if let Err(error) = stopped {
            // A failed stop must not make this caller wait on still-open pipes.
            self.reaper
                .retire(child, Readers::Rpc(std::mem::take(&mut self.readers)));
            return Err(error);
        }

        let mut result = Ok(());
        for reader in self.readers.drain(..) {
            let joined = reader
                .join()
                .map_err(|_| failure("A server output reader stopped unexpectedly"));
            result = finish(result, joined, "Server reader cleanup");
        }
        result
    }
}

impl Drop for Managed {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            self.reaper
                .retire(child, Readers::Rpc(std::mem::take(&mut self.readers)));
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::Managed;
    use crate::error::Result;
    use std::process::Command;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn dropping_a_process_does_not_wait_for_its_output_reader() -> Result<()> {
        let mut process = Managed::spawn(Command::new("sleep").arg("30"))?;
        let (release, blocked) = mpsc::channel();
        let (finished, completion) = mpsc::channel();
        process.track_reader(std::thread::spawn(move || {
            let _ = blocked.recv();
            let _ = finished.send(());
        }));

        let (dropped, observed) = mpsc::channel();
        let dropping = std::thread::spawn(move || {
            drop(process);
            let _ = dropped.send(());
        });
        let dropped_without_reader = observed.recv_timeout(Duration::from_secs(2));

        // Always release the reader before asserting, including when Drop regresses.
        release.send(())?;
        completion.recv_timeout(Duration::from_secs(5))?;
        dropping
            .join()
            .expect("the process owner must finish dropping");
        dropped_without_reader.expect("dropping must hand off a blocked reader without waiting");
        Ok(())
    }

    #[test]
    fn explicit_close_reports_a_failed_reader_and_is_idempotent() -> Result<()> {
        let mut process = Managed::spawn(Command::new("sleep").arg("30"))?;
        process.track_reader(std::thread::spawn(|| {
            panic!("simulated output reader failure")
        }));

        let error = process
            .close()
            .expect_err("a failed output reader must be reported");
        assert!(error.to_string().contains("reader"), "{error}");
        assert!(
            process.id().is_none(),
            "close must relinquish the stopped child"
        );
        process.close()?;
        Ok(())
    }
}
