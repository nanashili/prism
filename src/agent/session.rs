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
use configuration::SessionRestore;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

pub(super) struct Session {
    rpc: Rpc,
    updates: super::updates::Updates,
    id: String,
    identity: SessionIdentity,
    images: bool,
    has_prompted: bool,
    permissions: super::permission::Policy,
    permission_requests: super::permission::Requests,
    pending: control::Requests,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionIdentity {
    chat: u64,
    command: String,
    arguments: String,
    registry_id: String,
    cwd: std::path::PathBuf,
    servers: Vec<serde_json::Value>,
    static_instructions: String,
    timeout_seconds: u64,
}

impl SessionIdentity {
    fn from_prompt(prompt: &Prompt) -> Self {
        Self {
            chat: prompt.chat,
            command: prompt.settings.agent.clone(),
            arguments: prompt.settings.agent_args.clone(),
            registry_id: prompt.settings.agent_id.clone(),
            cwd: prompt.cwd.clone(),
            servers: prompt.servers.clone(),
            static_instructions: prompt.context.static_instructions.clone(),
            timeout_seconds: prompt.settings.timeout_seconds,
        }
    }

    fn matches_prompt(&self, prompt: &Prompt) -> bool {
        self.chat == prompt.chat
            && self.command == prompt.settings.agent
            && self.arguments == prompt.settings.agent_args
            && self.registry_id == prompt.settings.agent_id
            && self.cwd == prompt.cwd
            && self.servers == prompt.servers
            && self.static_instructions == prompt.context.static_instructions
            && self.timeout_seconds == prompt.settings.timeout_seconds
    }
}

impl Session {
    pub(super) fn close(&mut self) -> Result<()> {
        self.permission_requests.dismiss_all();
        self.rpc.close()
    }

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
    let reuse = current
        .as_ref()
        .is_some_and(|session| session.identity.matches_prompt(prompt));
    if !reuse {
        if let Some(mut session) = current.take() {
            session.close()?;
        }
        *current = Some(startup::connect(
            prompt, sink, cancelled, shutdown, controls,
        )?);
    }
    let session = current
        .as_mut()
        .ok_or("The agent session was not created")?;
    session.permissions = super::permission::Policy::new(prompt);
    session.configure(
        prompt,
        SessionRestore::from(!reuse),
        sink,
        cancelled,
        controls,
    )
}

#[cfg(test)]
mod tests {
    use super::SessionIdentity;
    use crate::agent::{Prompt, PromptContext};
    use crate::settings::Settings;
    use std::path::PathBuf;

    fn prompt() -> Prompt {
        Prompt {
            chat: 1,
            project: None,
            file: None,
            text: "question".into(),
            attachments: Vec::new(),
            context: PromptContext {
                static_instructions: "static".into(),
                dynamic_context: "first view".into(),
            },
            history: String::new(),
            cwd: PathBuf::from("/tmp/project"),
            settings: Settings::default(),
            servers: Vec::new(),
            picks: Vec::new(),
        }
    }

    #[test]
    fn identity_reuses_dynamic_context_but_invalidates_static_and_timeout_changes() {
        let original = prompt();
        let identity = SessionIdentity::from_prompt(&original);

        let mut dynamic_change = prompt();
        dynamic_change.context.dynamic_context = "second view".into();
        assert!(identity.matches_prompt(&dynamic_change));

        let mut static_change = prompt();
        static_change.context.static_instructions = "new static".into();
        assert!(!identity.matches_prompt(&static_change));

        let mut timeout_change = prompt();
        timeout_change.settings.timeout_seconds += 1;
        assert!(!identity.matches_prompt(&timeout_change));
    }
}
