mod child;

use super::activity::prefix;
use super::agent_updates::ChildUpdate;
use super::{ContextUsage, Outcome, PlanItem, ToolStep};
use serde::{Deserialize, Serialize};

const MAX_BYTES: usize = 128 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct TurnProgress {
    pub plan: Vec<PlanItem>,
    pub children: Vec<Child>,
    pub usage: Option<ContextUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<super::Goal>,
    pub limited: bool,
    #[serde(skip)]
    recent_child: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Child {
    pub id: String,
    pub parent: Option<String>,
    pub name: String,
    pub task: String,
    pub status: String,
    pub text: String,
    pub thought: String,
    pub tools: Vec<ToolStep>,
}

impl TurnProgress {
    pub fn update_child(&mut self, update: ChildUpdate) {
        let existing = self.children.iter().position(|child| child.id == update.id);
        let index = match existing {
            Some(index) => index,
            None => {
                if self.children.len() >= 64 {
                    self.children.remove(0);
                    self.limited = true;
                }
                self.children.push(Child {
                    id: prefix(&update.id, 240).into(),
                    parent: update.parent.map(|parent| prefix(&parent, 240).into()),
                    name: "Agent".into(),
                    task: String::new(),
                    status: "running".into(),
                    text: String::new(),
                    thought: String::new(),
                    tools: Vec::new(),
                });
                self.children.len() - 1
            }
        };
        let child = &mut self.children[index];
        if let Some(name) = update.name {
            child.name = prefix(&name, 240).into();
        }
        if let Some(task) = update.task {
            child.task = prefix(&task, 1000).into();
        }
        if let Some(status) = update.status {
            child.status = prefix(&status, 64).into();
        }
        if let Some(event) = update.event {
            self.limited |= child.apply(event);
        }
        self.recent_child
            .get_or_insert_default()
            .clone_from(&child.id);
        self.retain_recent();
    }

    pub(in crate::model) fn retain_recent(&mut self) {
        let mut bytes = self.retained_bytes();
        let recent = self
            .children
            .iter()
            .position(|child| Some(&child.id) == self.recent_child.as_ref())
            .or_else(|| self.children.len().checked_sub(1));
        for (index, child) in self.children.iter_mut().enumerate() {
            if bytes <= MAX_BYTES {
                break;
            }
            if Some(index) == recent {
                continue;
            }
            let before = child.retained_bytes();
            child.text = String::new();
            child.thought = String::new();
            child.tools = Vec::new();
            bytes -= before - child.retained_bytes();
            self.limited = true;
        }

        let mut count = self.children.len();
        let mut index = 0;
        self.children.retain(|child| {
            let keep = Some(index) == recent || (bytes <= MAX_BYTES && count <= 64);
            index += 1;
            if !keep {
                bytes -= child.retained_bytes();
                count -= 1;
            }
            keep
        });
        if index > count {
            self.limited = true;
        }
    }

    pub fn finish(&mut self) {
        for child in &mut self.children {
            if matches!(child.status.as_str(), "running" | "waiting" | "unknown") {
                child.status = "disconnected".into();
            }
            for tool in &mut child.tools {
                if tool.status == Outcome::Running {
                    tool.status = Outcome::Stopped;
                    tool.ended = Some(super::now_ms());
                }
            }
        }
    }

    pub fn retained_bytes(&self) -> usize {
        let plan: usize = self
            .plan
            .iter()
            .map(|entry| entry.content.len() + entry.status.len())
            .sum();
        plan + self.goal.as_ref().map_or(0, |goal| goal.objective.len())
            + self.recent_child.as_ref().map_or(0, String::len)
            + self
                .children
                .iter()
                .map(Child::retained_bytes)
                .sum::<usize>()
    }
}
