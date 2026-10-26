#[cfg(unix)]
use super::signal;
use super::terminate;
use crate::error::{Result, failure};
use std::process::Child;
use std::sync::{OnceLock, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

/// Started before a child is launched, so dropping an RPC can hand off waiting
/// without needing to allocate a new operating-system thread during teardown.
pub(super) struct Reaper {
    sender: mpsc::Sender<Retired>,
}

struct Retired {
    child: Option<Child>,
    readers: Readers,
    reported_failure: Option<String>,
}

pub(super) enum Readers {
    Rpc(Vec<JoinHandle<()>>),
}

impl Reaper {
    pub(super) fn start() -> Result<Self> {
        static QUEUE: OnceLock<std::io::Result<mpsc::Sender<Retired>>> = OnceLock::new();
        let queue = QUEUE.get_or_init(|| {
            let (sender, receiver) = mpsc::channel::<Retired>();
            std::thread::Builder::new()
                .name("prism-process-reaper".into())
                .spawn(move || reap_pending(&receiver))?;
            Ok(sender)
        });
        match queue {
            Ok(sender) => Ok(Self {
                sender: sender.clone(),
            }),
            Err(error) => Err(failure(format!("Cannot start process cleanup: {error}"))),
        }
    }

    pub(super) fn retire(&self, child: Child, readers: Readers) {
        // Signal immediately, rather than leaving a running helper behind a
        // different helper's pending wait in the cleanup queue.
        #[cfg(unix)]
        let mut child = child;
        #[cfg(unix)]
        if let Err(error) = signal(&mut child) {
            eprintln!(
                "Could not signal process {} during cleanup: {error}",
                child.id()
            );
        }
        let retired = Retired {
            child: Some(child),
            readers,
            reported_failure: None,
        };
        if let Err(error) = self.sender.send(retired) {
            // The queue has a process-long sender and its worker has no panicking
            // operations. A disconnected queue therefore indicates an internal
            // failure; report it and make one nonblocking attempt to reap.
            let mut retired = error.0;
            let Some(child) = retired.child.as_mut() else {
                return;
            };
            match child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => eprintln!("Process cleanup stopped before reaping {}", child.id()),
                Err(error) => eprintln!("Cannot reap process {}: {error}", child.id()),
            }
        }
    }
}

impl Retired {
    fn stop(&mut self) -> Result<()> {
        if let Some(child) = &mut self.child {
            terminate(child)?;
            self.child = None;
        }
        Ok(())
    }

    fn report_failure(&mut self, error: &dyn std::error::Error) {
        let message = error.to_string();
        if self.reported_failure.as_ref() != Some(&message) {
            let id = self.child.as_ref().map(Child::id);
            eprintln!("Could not reap server process {id:?}; cleanup will retry: {message}");
            self.reported_failure = Some(message);
        }
    }
}

fn reap_pending(receiver: &mpsc::Receiver<Retired>) {
    let mut pending = Vec::<Retired>::new();
    loop {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(retired) => pending.push(retired),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            // The process-long sender remains in QUEUE until process exit.
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }

        let mut waiting = Vec::new();
        for mut retired in pending {
            match retired.stop() {
                Ok(()) if retired.readers.is_finished() => retired.readers.join(),
                Ok(()) => waiting.push(retired),
                Err(error) => {
                    retired.report_failure(error.as_ref());
                    waiting.push(retired);
                }
            }
        }
        pending = waiting;
    }
}

impl Readers {
    fn is_finished(&self) -> bool {
        match self {
            Self::Rpc(readers) => readers.iter().all(JoinHandle::is_finished),
        }
    }

    fn join(self) {
        match self {
            Self::Rpc(readers) => {
                for reader in readers {
                    if reader.join().is_err() {
                        eprintln!("A server output reader failed during cleanup");
                    }
                }
            }
        }
    }
}
