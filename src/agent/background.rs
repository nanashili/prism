mod worker;

use super::{options, session};
use crate::error::{Result, failure};
use crate::events::Sink;
use crate::settings::Settings;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

pub(crate) struct Prompt {
    pub chat: u64,
    pub project: Option<String>,
    pub file: Option<PathBuf>,
    pub text: String,
    pub attachments: Vec<crate::model::Attachment>,
    pub context: PromptContext,
    pub history: String,
    pub cwd: PathBuf,
    pub settings: Settings,
    pub servers: Vec<serde_json::Value>,
    pub picks: Vec<options::Pick>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PromptContext {
    pub static_instructions: String,
    pub dynamic_context: String,
}

pub(super) enum Request {
    Prepare(Prompt),
    Prompt(Prompt),
}

pub(crate) struct Agent {
    sender: Option<mpsc::SyncSender<Request>>,
    preparing: Arc<AtomicBool>,
    controls: mpsc::SyncSender<session::Control>,
    cancelled: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Agent {
    pub fn start(sink: Sink) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Request>(2);
        let (controls, commands) = mpsc::sync_channel(8);
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let preparing = Arc::new(AtomicBool::new(false));
        let preparation = preparing.clone();
        let thread = std::thread::spawn(move || {
            worker::run(receiver, commands, sink, flag, stop, preparation);
        });
        Self {
            sender: Some(sender),
            preparing,
            controls,
            cancelled,
            stopped,
            thread: Some(thread),
        }
    }

    pub fn prompt(&self, prompt: Prompt) -> Result<()> {
        self.cancelled.store(false, Ordering::Relaxed);
        self.sender
            .as_ref()
            .ok_or("The agent worker has closed")?
            .try_send(Request::Prompt(prompt))
            .map_err(|_| failure("The agent is already working"))
    }

    pub fn prepare(&self, prompt: Prompt) -> Result<()> {
        if self.preparing.swap(true, Ordering::Relaxed) {
            return Ok(());
        }
        self.cancelled.store(false, Ordering::Relaxed);
        let result = self
            .sender
            .as_ref()
            .ok_or_else(|| failure("The agent worker has closed"))
            .and_then(|sender| {
                sender
                    .try_send(Request::Prepare(prompt))
                    .map_err(|_| failure("The agent is already connecting"))
            });
        if result.is_err() {
            self.preparing.store(false, Ordering::Relaxed);
        }
        result
    }

    pub fn is_preparing(&self) -> bool {
        self.preparing.load(Ordering::Relaxed)
    }

    pub fn set_option(&self, pick: options::Pick) -> Result<()> {
        self.controls
            .try_send(session::Control::Option(pick))
            .map_err(|_| failure("The agent has too many pending changes"))
    }

    pub fn control_goal(&self, action: &'static str) -> Result<()> {
        self.controls
            .try_send(session::Control::Goal(action))
            .map_err(|_| failure("The agent has too many pending changes"))
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn stop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.cancel();
        self.sender.take();
    }

    pub fn close(mut self) -> Result<()> {
        self.stop();
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| failure("The agent worker stopped unexpectedly"))?;
        }
        Ok(())
    }

    pub fn has_exited(&self) -> bool {
        self.thread
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished)
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        self.stop();
        if self
            .thread
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
            && let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("The agent worker stopped unexpectedly");
        }
    }
}
