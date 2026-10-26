use super::framing::Frame;
use crate::error::{Result, failure};
use serde_json::Value;
use std::cell::Cell;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::Duration;

const MAX_MESSAGES: usize = 16;
const MAX_QUEUED_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn channel() -> (Sender, Receiver) {
    let (sender, receiver) = mpsc::sync_channel(MAX_MESSAGES);
    let budget = Arc::new(Budget::default());
    (
        Sender {
            sender,
            budget: budget.clone(),
        },
        Receiver {
            receiver,
            budget,
            failed: Cell::new(false),
        },
    )
}

pub(super) struct Sender {
    sender: mpsc::SyncSender<Result<Queued>>,
    budget: Arc<Budget>,
}

pub(super) struct Receiver {
    receiver: mpsc::Receiver<Result<Queued>>,
    budget: Arc<Budget>,
    failed: Cell<bool>,
}

struct Queued {
    frame: Frame,
    permit: Permit,
}

#[derive(Default)]
struct Budget {
    state: Mutex<State>,
    available: Condvar,
}

#[derive(Default)]
struct State {
    bytes: usize,
    closed: bool,
}

struct Permit {
    budget: Arc<Budget>,
    bytes: usize,
}

impl Sender {
    pub(super) fn send(&self, frame: Result<Frame>) -> Result<()> {
        let queued = match frame {
            Ok(frame) => {
                let permit = self.budget.reserve(frame.capacity())?;
                Ok(Queued { frame, permit })
            }
            Err(error) => Err(error),
        };
        self.sender
            .send(queued)
            .map_err(|_| failure("The server reader closed"))
    }
}

impl Receiver {
    #[cfg(test)]
    pub(super) fn retained_bytes(&self) -> usize {
        self.budget
            .state
            .lock()
            .expect("test queue state is available")
            .bytes
    }

    pub(super) fn poll(&self, timeout: Duration) -> Result<Option<Value>> {
        if self.failed.get() {
            return Err(failure("The server process closed its output"));
        }
        let result = match self.receiver.recv_timeout(timeout) {
            Ok(Ok(Queued { frame, permit })) => {
                // The consumer owns this frame now; parsing cannot make the queue
                // retain a large decoded JSON tree behind a slow caller.
                drop(permit);
                frame.decode().map(Some)
            }
            Ok(Err(error)) => Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => return Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(failure("The server process closed its output"))
            }
        };
        if result.is_err() {
            // A malformed JSON frame remains fatal, as it was in the eager reader.
            self.failed.set(true);
            self.budget.close();
            while self.receiver.try_recv().is_ok() {}
        }
        result
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.budget.close();
    }
}

impl Budget {
    fn reserve(self: &Arc<Self>, bytes: usize) -> Result<Permit> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| failure("The server queue is unavailable"))?;
        // Keep every valid protocol frame admissible. An oversized frame runs
        // alone; the producer can additionally hold one frame awaiting admission.
        while !state.closed
            && state.bytes != 0
            && bytes > MAX_QUEUED_BYTES.saturating_sub(state.bytes)
        {
            state = self
                .available
                .wait(state)
                .map_err(|_| failure("The server queue is unavailable"))?;
        }
        if state.closed {
            return Err(failure("The server reader closed"));
        }
        state.bytes += bytes;
        Ok(Permit {
            budget: self.clone(),
            bytes,
        })
    }

    fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
        }
        self.available.notify_all();
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        if let Ok(mut state) = self.budget.state.lock() {
            state.bytes -= self.bytes;
        }
        self.budget.available.notify_all();
    }
}

#[cfg(test)]
mod tests;
