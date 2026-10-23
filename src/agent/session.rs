mod configuration;
mod control;
mod exchange;
mod startup;
mod turn;

pub(super) use control::{Control, unavailable};

use super::Prompt;
use crate::error::Result;
use crate::events::Sink;
use crate::protocol::Rpc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

pub(super) struct Session {
    rpc: Rpc,
    updates: super::updates::Updates,
    id: String,
    chat: u64,
    command: String,
    arguments: String,
    registry_id: String,
    cwd: std::path::PathBuf,
    servers: Vec<serde_json::Value>,
    images: bool,
    has_prompted: bool,
    permissions: super::permission::Policy,
    permission_requests: super::permission::Requests,
    pending: control::Requests,
}

impl Session {
    pub(super) fn has_active_goal(&self) -> bool {
        self.updates.goal_active
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.permission_requests.dismiss_all();
    }
}

pub(super) fn run(
    current: &mut Option<Session>,
    prompt: Prompt,
    sink: &Sink,
    cancelled: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    controls: &Receiver<Control>,
) -> Result<()> {
    prepare(current, &prompt, sink, &cancelled, shutdown, controls)?;
    let session = current
        .as_mut()
        .ok_or("The agent session was not created")?;
    let history = if session.has_prompted {
        ""
    } else {
        &prompt.history
    };
    if !prompt.text.trim_start().starts_with('/') {
        session.has_prompted = true;
    }
    session.prompt(&prompt, history, sink, &cancelled, controls)
}

pub(super) fn prepare(
    current: &mut Option<Session>,
    prompt: &Prompt,
    sink: &Sink,
    cancelled: &AtomicBool,
    shutdown: Arc<AtomicBool>,
    controls: &Receiver<Control>,
) -> Result<()> {
    let reuse = current.as_ref().is_some_and(|session| {
        session.chat == prompt.chat
            && session.command == prompt.settings.agent
            && session.arguments == prompt.settings.agent_args
            && session.registry_id == prompt.settings.agent_id
            && session.cwd == prompt.cwd
            && session.servers == prompt.servers
    });
    if !reuse {
        *current = None;
        *current = Some(startup::connect(
            prompt, sink, cancelled, shutdown, controls,
        )?);
    }
    let session = current
        .as_mut()
        .ok_or("The agent session was not created")?;
    session.permissions = super::permission::Policy::new(prompt);
    session.configure(prompt, !reuse, sink, cancelled, controls)
}
