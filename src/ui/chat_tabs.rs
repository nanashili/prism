#[path = "chat_tabs/operations.rs"]
mod operations;
#[path = "chat_tabs/project.rs"]
mod project;
#[path = "chat_tabs/routing.rs"]
mod routing;

use super::Chat;
use crate::{agent::Agent, events::Sink, settings::Settings};

pub(in crate::ui) struct Tabs {
    pub active: u64,
    pub next: u64,
    pub order: Vec<u64>,
    pub parked: Vec<OpenTab>,
    pub sink: Sink,
    pub inspector: bool,
    pub retired: Vec<Agent>,
    pub history: crate::ui::agent_dock::history::History,
    pub inspection: crate::ui::agent_dock::inspector::Inspection,
}

pub(in crate::ui) struct OpenTab {
    pub key: u64,
    pub chat: Chat,
    pub agent: Option<Agent>,
    pub choice: AgentChoice,
}

pub(in crate::ui) struct AgentChoice {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: String,
}

impl Tabs {
    pub fn new(sink: Sink) -> Self {
        Self {
            active: 0,
            next: 1,
            order: vec![0],
            parked: Vec::new(),
            sink,
            inspector: false,
            retired: Vec::new(),
            history: Default::default(),
            inspection: Default::default(),
        }
    }

    pub fn any_busy(&self, active: &Chat) -> bool {
        active.busy || self.parked.iter().any(|tab| tab.chat.busy)
    }
}

impl AgentChoice {
    pub fn capture(settings: &Settings) -> Self {
        Self {
            id: settings.agent_id.clone(),
            name: settings.agent_name.clone(),
            command: settings.agent.clone(),
            args: settings.agent_args.clone(),
        }
    }

    pub fn exchange(&mut self, settings: &mut Settings) {
        std::mem::swap(&mut self.id, &mut settings.agent_id);
        std::mem::swap(&mut self.name, &mut settings.agent_name);
        std::mem::swap(&mut self.command, &mut settings.agent);
        std::mem::swap(&mut self.args, &mut settings.agent_args);
    }
}
