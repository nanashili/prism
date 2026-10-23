use crate::error::Result;
use crate::events::{Event, Sink};
use crate::protocol::{Framing, Rpc};
use crate::settings::Settings;
use serde_json::{Value, json};
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
        let mut command = Command::new(&settings.lldb);
        let rpc = Rpc::spawn(&mut command, Framing::Dap, cancelled)?;
        let mut debugger = Debugger { rpc, sink };
        debugger.request(
            "initialize",
            json!({
                "adapterID":"lldb-dap", "linesStartAt1":true, "columnsStartAt1":true,
                "pathFormat":"path", "supportsRunInTerminalRequest":false
            }),
        )?;
        *current = Some(debugger);
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
    fn request(&mut self, command: &str, args: Value) -> Result<Value> {
        let sink = self.sink.clone();
        self.rpc
            .call(command, args, Duration::from_secs(30), |event| {
                publish(&sink, event)?;
                Ok(None)
            })
    }

    fn start(&mut self, command: &str, args: Value) -> Result<()> {
        let id = self.rpc.begin(command, args)?;
        self.request("configurationDone", json!({}))?;
        let sink = self.sink.clone();
        self.rpc
            .wait(id, command, Duration::from_secs(30), |event| {
                publish(&sink, event)?;
                Ok(None)
            })?;
        Ok(())
    }
}

fn publish(sink: &Sink, event: &Value) -> Result<()> {
    if let Some(kind) = event["event"].as_str() {
        sink.send(Event::Progress(format!("Debugger: {kind}")))?;
    }
    Ok(())
}
