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
        restart: bool,
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
    #[cfg_attr(feature = "profiling", hotpath::measure)]
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
        match &mut self.state {
            State::Running {
                cancelled, restart, ..
            } => {
                cancelled.store(true, Ordering::Relaxed);
                *restart = true;
            }
            _ => self.state = State::Idle,
        }
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    fn receive(&mut self) {
        let State::Running {
            receiver, restart, ..
        } = &self.state
        else {
            return;
        };
        let restart = *restart;
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
                if restart { State::Idle } else { State::Ready }
            }
            Err(error) => {
                if restart {
                    State::Idle
                } else {
                    State::Failed(error)
                }
            }
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
                // A closed receiver means the measurement was dropped or the app exited.
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
                    restart: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidation_keeps_cancelled_scan_owned_until_it_finishes() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut measurement = Measurement {
            report: None,
            state: State::Running {
                receiver,
                cancelled: Arc::clone(&cancelled),
                restart: false,
            },
        };

        measurement.invalidate();

        assert!(cancelled.load(Ordering::Relaxed));
        measurement.update(&egui::Context::default());

        sender
            .send(Err("cancelled".into()))
            .expect("the cancelled scan receiver remains owned until it finishes");
        measurement.receive();

        assert!(matches!(measurement.state, State::Idle));
    }
}
