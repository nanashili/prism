use super::{ToolUpdate, TurnProgress};
use serde::{Deserialize, Serialize};

pub(crate) enum AgentUpdate {
    Commands(Vec<SlashCommand>),
    Plan(Vec<PlanItem>),
    Usage(ContextUsage),
    Child(ChildUpdate),
    Notice(String),
    Goal(Option<super::Goal>),
    SessionClosed,
    OptionRejected {
        id: String,
        value: String,
        reason: String,
    },
}

#[derive(Clone)]
pub(crate) struct SlashCommand {
    pub name: String,
    pub description: String,
    pub hint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlanItem {
    pub content: String,
    pub status: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct ContextUsage {
    pub used: u64,
    pub size: u64,
    pub cost: Option<f64>,
    pub currency: String,
}

pub(crate) struct ChildUpdate {
    pub id: String,
    pub parent: Option<String>,
    pub name: Option<String>,
    pub task: Option<String>,
    pub status: Option<String>,
    pub event: Option<ChildEvent>,
}

pub(crate) enum ChildEvent {
    Text(String),
    Thought(String),
    Tool(ToolUpdate),
}

impl AgentUpdate {
    pub fn apply_to_turn(self, progress: &mut TurnProgress) {
        match self {
            Self::Plan(plan) => progress.plan = plan,
            Self::Usage(usage) => progress.usage = Some(usage),
            Self::Child(child) => progress.update_child(child),
            Self::Goal(goal) => progress.goal = goal,
            Self::Commands(_)
            | Self::Notice(_)
            | Self::OptionRejected { .. }
            | Self::SessionClosed => {}
        }
    }
}
