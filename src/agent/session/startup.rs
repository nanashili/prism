use super::Session;
use crate::agent::{Prompt, catalog, codex, launch, permission, updates};
use crate::error::{Result, failure};
use crate::events::{Event, Sink};
use crate::protocol::{Framing, Rpc};
use serde_json::json;
use std::process::Command;
use std::sync::{Arc, atomic::AtomicBool, mpsc::Receiver};
use std::time::Duration;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(super) fn connect(
    prompt: &Prompt,
    sink: &Sink,
    cancelled: &AtomicBool,
    shutdown: Arc<AtomicBool>,
    controls: &Receiver<super::Control>,
) -> Result<Session> {
    sink.send(Event::AgentUpdate(crate::model::AgentUpdate::Commands(
        Vec::new(),
    )))?;
    let mut command = launch_command(prompt, cancelled)?;
    command.current_dir(&prompt.cwd);
    let rpc = Rpc::spawn(&mut command, Framing::Lines, shutdown)?;
    let mut session = Session {
        rpc,
        updates: updates::Updates::new(crate::agent::picks::key(&prompt.settings)),
        id: String::new(),
        identity: super::SessionIdentity::from_prompt(prompt),
        permissions: permission::Policy::new(prompt),
        permission_requests: Default::default(),
        pending: Default::default(),
        has_prompted: false,
        images: false,
    };
    session.initialize(prompt, sink, cancelled, controls)?;
    Ok(session)
}

impl Session {
    fn initialize(
        &mut self,
        prompt: &Prompt,
        sink: &Sink,
        cancelled: &AtomicBool,
        controls: &Receiver<super::Control>,
    ) -> Result<()> {
        let initialized = self.call(
            "initialize",
            json!({
                "protocolVersion":1,
                "clientInfo":{"name":"Prism","version":env!("CARGO_PKG_VERSION")},
                "clientCapabilities":codex::capabilities(prompt)
            }),
            Duration::from_secs(60),
            sink,
            cancelled,
            controls,
        )?;
        if initialized["protocolVersion"] != 1 {
            return Err(failure(
                "This agent requires an unsupported ACP protocol version",
            ));
        }
        self.images = initialized["agentCapabilities"]["promptCapabilities"]["image"]
            .as_bool()
            .unwrap_or(false);

        let created = self.call(
            "session/new",
            codex::session(prompt),
            Duration::from_secs(60),
            sink,
            cancelled,
            controls,
        )?;
        self.id = created["sessionId"]
            .as_str()
            .ok_or("The agent did not return a session identifier")?
            .to_owned();
        self.updates.root = Some(self.id.clone());
        self.updates.publish_options(created, sink)
    }
}

fn launch_command(prompt: &Prompt, cancelled: &AtomicBool) -> Result<Command> {
    let settings = &prompt.settings;
    let mut launch = if settings.agent_id.is_empty() {
        catalog::Launch {
            command: settings.agent.clone(),
            args: shlex::split(&settings.agent_args)
                .ok_or("The agent arguments contain unclosed quotes")?,
            env: Default::default(),
        }
    } else {
        let entry = launch::selected(&settings.agent_id, cancelled)?;
        launch::resolve(&entry, cancelled)?
    };

    codex::configure_launch(&mut launch, prompt)?;

    let mut command = Command::new(launch.command);
    if std::path::Path::new(command.get_program())
        .file_stem()
        .is_some_and(|name| name == "npx")
    {
        // An explicit prefix prevents npm from scanning unrelated globally linked packages.
        // Configured arguments follow this default so an explicit user prefix takes precedence.
        command
            .arg("--prefix")
            .arg(&prompt.cwd)
            .env("npm_config_prefer_offline", "true");
    }
    command.args(launch.args).envs(launch.env);
    Ok(command)
}
