use super::ExportRunner;
use super::Job;
use super::RuntimeSnapshot;
use crate::error::{Result, failure};
use crate::events::{Event, Sink};
use crate::model::Project;
use crate::settings::Settings;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};

pub(crate) struct Worker {
    sender: Option<super::WorkerPort>,
    cancelled: Arc<AtomicBool>,
    stop_generation: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    runtime: RuntimeSnapshot,
}

pub(super) struct State {
    pub(super) store: crate::storage::Store,
    pub(super) settings: Settings,
    pub(super) engine: Option<crate::analysis::Engine>,
    pub(super) project: Option<Project>,
    pub(super) cancelled: Arc<AtomicBool>,
    pub(super) stop_generation: Arc<AtomicU64>,
    pub(super) observed_stop: u64,
    pub(super) sink: Sink,
    pub(super) debugger: Option<crate::debug::Debugger>,
    pub(super) debug_project: Option<String>,
    pub(super) sidebar_counts: Option<(String, crate::storage::navigation::Counts)>,
    pub(super) decompilation: Option<ExportRunner>,
    pub(super) runtime: RuntimeSnapshot,
}

impl Worker {
    pub fn start(settings: Settings, sink: Sink) -> Self {
        let (ordinary, receiver) = mpsc::sync_channel(8);
        let (controls, control_receiver) = mpsc::sync_channel(4);
        let sender = super::WorkerPort { ordinary, controls };
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let stop_generation = Arc::new(AtomicU64::new(0));
        let worker_stops = stop_generation.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let runtime = RuntimeSnapshot::new();
        let worker_runtime = runtime.clone();
        let thread = std::thread::spawn(move || {
            let store =
                match crate::storage::Store::open(&crate::settings::support_dir().join("prism.db"))
                {
                    Ok(store) => store,
                    Err(error) => {
                        if let Err(send_error) = sink.send(Event::Finished(Err(error))) {
                            eprintln!("{send_error}");
                        }
                        return;
                    }
                };
            let mut state = State {
                store,
                settings,
                engine: None,
                project: None,
                cancelled: flag,
                stop_generation: worker_stops,
                observed_stop: 0,
                sink,
                debugger: None,
                debug_project: None,
                sidebar_counts: None,
                decompilation: None,
                runtime: worker_runtime,
            };
            state.run(receiver, control_receiver, &stop);
        });
        Self {
            sender: Some(sender),
            cancelled,
            stop_generation,
            stopped,
            thread: Some(thread),
            runtime,
        }
    }

    pub fn submit(&self, job: Job) -> Result<()> {
        self.sender
            .as_ref()
            .ok_or("The worker is closed")?
            .submit(job)
    }

    pub fn cancel(&self) {
        self.stop_generation.fetch_add(1, Ordering::Release);
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn port(&self) -> Result<super::WorkerPort> {
        self.sender
            .clone()
            .ok_or_else(|| failure("The analysis worker is closed"))
    }

    pub fn runtime(&self) -> RuntimeSnapshot {
        self.runtime.clone()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.cancel();
        self.sender.take();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("The analysis worker stopped unexpectedly");
        }
    }
}
