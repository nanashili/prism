use crate::storage::usage::{self, Usage};
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Running {
        receiver: Receiver<Result<Usage, String>>,
        cancelled: Arc<AtomicBool>,
    },
    Ready,
    Failed(String),
}

#[derive(Default)]
pub(super) struct Measurement {
    pub report: Option<Usage>,
    state: State,
}

impl Measurement {
    pub fn update(&mut self, context: &egui::Context) {
        self.receive();
        if matches!(self.state, State::Idle) {
            self.start(context);
        }
    }

    pub fn loading(&self) -> bool {
        matches!(self.state, State::Running { .. })
    }

    pub fn error(&self) -> Option<&str> {
        match &self.state {
            State::Failed(error) => Some(error),
            _ => None,
        }
    }

    pub fn invalidate(&mut self) {
        self.cancel();
        self.state = State::Idle;
    }

    fn receive(&mut self) {
        let State::Running { receiver, .. } = &self.state else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                Err("The storage scan stopped before finishing.".into())
            }
        };
        self.state = match result {
            Ok(report) => {
                self.report = Some(report);
                State::Ready
            }
            Err(error) => State::Failed(error),
        };
    }

    fn start(&mut self, context: &egui::Context) {
        let (sender, receiver) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let scan_cancelled = Arc::clone(&cancelled);
        let context = context.clone();
        let thread = std::thread::Builder::new()
            .name("storage-usage".into())
            .spawn(move || {
                let report = usage::measure(&crate::settings::support_dir(), &scan_cancelled)
                    .map_err(|error| error.to_string());
                // A closed receiver means the page invalidated the scan or the app exited.
                if sender.send(report).is_ok() {
                    context.request_repaint();
                }
            });
        self.state = match thread {
            Ok(thread) => {
                // The scan owns no application state and stops when cancellation is observed.
                drop(thread);
                State::Running {
                    receiver,
                    cancelled,
                }
            }
            Err(error) => State::Failed(format!("Could not start measuring storage: {error}")),
        };
    }

    fn cancel(&self) {
        if let State::Running { cancelled, .. } = &self.state {
            cancelled.store(true, Ordering::Relaxed);
        }
    }
}

impl Drop for Measurement {
    fn drop(&mut self) {
        self.cancel();
    }
}
