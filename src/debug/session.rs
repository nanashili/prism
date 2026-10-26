use crate::analysis::execution::{Observation, ObservationKind};
use crate::error::Result;
use crate::events::{Event, Sink};
use crate::protocol::{Framing, Rpc};
use crate::settings::Settings;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub(crate) enum Action {
    Launch { path: PathBuf, args: String },
    Attach(u32),
    Request { command: String, args: Value },
    Stop,
}

pub(crate) struct Debugger {
    rpc: Rpc,
    sink: Sink,
    project: Option<String>,
    stopped: StoppedThreads,
}

#[derive(Default)]
enum StoppedThreads {
    #[default]
    Running,
    All,
    Selected(BTreeSet<u64>),
}

pub(crate) fn execute(
    current: &mut Option<Debugger>,
    action: Action,
    settings: &Settings,
    cancelled: Arc<AtomicBool>,
    sink: Sink,
) -> Result<String> {
    if matches!(action, Action::Stop) {
        if let Some(debugger) = current {
            debugger.request("disconnect", json!({"terminateDebuggee":false}))?;
        }
        *current = None;
        return Ok("Disconnected".into());
    }
    if matches!(action, Action::Launch { .. } | Action::Attach(_)) {
        *current = None;
        *current = Some(Debugger::connect(settings, cancelled, sink)?);
    }
    let debugger = current
        .as_mut()
        .ok_or("Launch or attach to a program first")?;
    match action {
        Action::Launch { path, args } => {
            let args = shlex::split(&args).ok_or("Invalid quoted program arguments")?;
            debugger.start(
                "launch",
                json!({"program":path,"args":args,"stopOnEntry":true}),
            )?;
            Ok("Program launched. Choose Threads to inspect the stopped process.".into())
        }
        Action::Attach(pid) => {
            debugger.start("attach", json!({"pid":pid}))?;
            Ok("Attached".into())
        }
        Action::Request { command, args } => Ok(serde_json::to_string_pretty(
            &debugger.request(&command, args)?,
        )?),
        Action::Stop => unreachable!(),
    }
}

impl Debugger {
    fn connect(settings: &Settings, cancelled: Arc<AtomicBool>, sink: Sink) -> Result<Self> {
        let mut command = Command::new(&settings.lldb);
        let rpc = Rpc::spawn(&mut command, Framing::Dap, cancelled)?;
        let mut debugger = Self {
            rpc,
            sink,
            project: None,
            stopped: StoppedThreads::default(),
        };

        debugger.request(
            "initialize",
            json!({
                "adapterID":"lldb-dap", "linesStartAt1":true, "columnsStartAt1":true,
                "pathFormat":"path", "supportsRunInTerminalRequest":false
            }),
        )?;
        Ok(debugger)
    }

    pub(crate) fn bind_project(&mut self, project: &str) {
        self.project = Some(project.to_owned());
    }

    pub(crate) fn capture_sample(&mut self, project: &str, thread: u64) -> Result<Observation> {
        if self.project.as_deref() != Some(project) {
            return Err(crate::error::failure(
                "Launch the open target in the UI before sampling; attached processes have no verified target identity",
            ));
        }
        // Drain pending stop/continue events and confirm the thread still exists.
        let threads = self.request("threads", json!({}))?;
        let exists = threads["threads"].as_array().is_some_and(|threads| {
            threads
                .iter()
                .any(|value| value["id"].as_u64() == Some(thread))
        });
        if !exists || !self.stopped.contains(thread) {
            return Err(crate::error::failure(
                "Pause the selected debugger thread in the UI before sampling",
            ));
        }
        let stack = self.request(
            "stackTrace",
            json!({"threadId":thread,"startFrame":0,"levels":1}),
        )?;
        if !self.stopped.contains(thread) {
            return Err(crate::error::failure(
                "The debugger thread resumed before sampling completed",
            ));
        }
        let address = stack
            .pointer("/stackFrames/0/instructionPointerReference")
            .and_then(Value::as_str)
            .ok_or("The debugger returned no instruction address")?;
        Ok(Observation {
            kind: ObservationKind::Sample,
            address: address.to_owned(),
            target: None,
            thread_id: thread,
            count: 1,
        })
    }

    fn request(&mut self, command: &str, args: Value) -> Result<Value> {
        if matches!(
            command,
            "continue" | "next" | "stepIn" | "stepOut" | "restart" | "goto" | "evaluate"
        ) {
            self.stopped = StoppedThreads::Running;
        }
        let sink = self.sink.clone();
        let stopped = &mut self.stopped;
        self.rpc
            .call(command, args, Duration::from_secs(30), |event| {
                stopped.observe(event);
                publish(&sink, event)?;
                Ok(None)
            })
    }

    fn start(&mut self, command: &str, args: Value) -> Result<()> {
        let id = self.rpc.begin(command, args)?;
        self.request("configurationDone", json!({}))?;
        let sink = self.sink.clone();
        let stopped = &mut self.stopped;
        self.rpc
            .wait(id, command, Duration::from_secs(30), |event| {
                stopped.observe(event);
                publish(&sink, event)?;
                Ok(None)
            })?;
        Ok(())
    }
}

impl StoppedThreads {
    fn contains(&self, thread: u64) -> bool {
        match self {
            Self::Running => false,
            Self::All => true,
            Self::Selected(threads) => threads.contains(&thread),
        }
    }

    fn observe(&mut self, event: &Value) {
        match event["event"].as_str() {
            Some("stopped") if event["body"]["allThreadsStopped"] == true => *self = Self::All,
            Some("stopped") => {
                if let Some(thread) = event["body"]["threadId"].as_u64() {
                    match self {
                        Self::Running => *self = Self::Selected(BTreeSet::from([thread])),
                        Self::Selected(threads) => {
                            threads.insert(thread);
                        }
                        Self::All => {}
                    }
                }
            }
            Some("continued" | "exited" | "terminated") => *self = Self::Running,
            _ => {}
        }
    }
}

fn publish(sink: &Sink, event: &Value) -> Result<()> {
    if let Some(kind) = event["event"].as_str() {
        sink.send(Event::Progress(format!("Debugger: {kind}")))?;
    }
    Ok(())
}
