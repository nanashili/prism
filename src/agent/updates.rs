use crate::error::Result;
use crate::events::{Event, Sink};
use crate::model::{AgentUpdate, ChildEvent, ChildUpdate, ContextUsage, PlanItem, SlashCommand};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Updates {
    pub root: Option<String>,
    pub options: Value,
    agent: String,
    pub goal_active: bool,
    children: BTreeMap<String, Option<String>>,
}

impl Updates {
    pub(super) fn new(agent: String) -> Self {
        Self {
            agent,
            ..Default::default()
        }
    }

    pub(super) fn publish_options(&mut self, options: Value, sink: &Sink) -> Result<()> {
        super::options::merge(&mut self.options, options.clone());
        if let Err(error) = super::models::remember(&self.agent, &self.options) {
            sink.send(Event::AgentUpdate(AgentUpdate::Notice(format!(
                "Couldn't save the model list: {error}"
            ))))?;
        }
        sink.send(Event::AgentReady(options))
    }

    pub fn receive(&mut self, params: &Value, sink: &Sink) -> Result<()> {
        let update = &params["update"];
        let source = params["sessionId"].as_str().unwrap_or_default();
        let kind = update["sessionUpdate"].as_str().unwrap_or_default();
        if matches!(
            kind,
            "subagent_update" | "subagent_spawned" | "subagent_state_update"
        ) {
            return self.child_metadata(source, update, sink);
        }
        if self.children.contains_key(source) {
            return self.child_event(source, kind, update, sink);
        }
        if self.root.as_deref().is_some_and(|root| root != source) {
            return Ok(());
        }
        self.root.get_or_insert_with(|| source.to_owned());
        self.root_event(kind, update, sink)
    }

    fn child_event(&self, source: &str, kind: &str, update: &Value, sink: &Sink) -> Result<()> {
        let event = match kind {
            "agent_message_chunk" => text(update).map(ChildEvent::Text),
            "agent_thought_chunk" => text(update).map(ChildEvent::Thought),
            "tool_call" | "tool_call_update" => {
                super::tool_updates::parse(update).map(ChildEvent::Tool)
            }
            _ => None,
        };
        if let Some(event) = event {
            sink.send(Event::AgentUpdate(AgentUpdate::Child(ChildUpdate {
                id: source.into(),
                parent: self.children[source].clone(),
                name: None,
                task: None,
                status: None,
                event: Some(event),
            })))?;
        }
        Ok(())
    }

    fn root_event(&mut self, kind: &str, update: &Value, sink: &Sink) -> Result<()> {
        match kind {
            "agent_message_chunk" => {
                if let Some(text) = text(update) {
                    sink.send(Event::AgentText(text))?;
                }
            }
            "agent_thought_chunk" => {
                if let Some(text) = text(update) {
                    sink.send(Event::AgentThought(text))?;
                }
            }
            "tool_call" | "tool_call_update" => {
                if let Some(tool) = super::tool_updates::parse(update) {
                    sink.send(Event::AgentTool(tool))?;
                }
            }
            "config_option_update" | "current_mode_update" => {
                let options = if kind == "config_option_update" {
                    json!({"configOptions":update["configOptions"]})
                } else {
                    json!({"modes":{"currentModeId":update["currentModeId"]}})
                };
                self.publish_options(options, sink)?;
            }
            "available_commands_update" => {
                sink.send(Event::AgentUpdate(AgentUpdate::Commands(commands(update))))?;
            }
            "session_info_update" => {
                let goal = match crate::model::GoalUpdate::from_metadata(&update["_meta"]) {
                    crate::model::GoalUpdate::Unchanged => return Ok(()),
                    crate::model::GoalUpdate::Clear => None,
                    crate::model::GoalUpdate::Set(goal) => Some(goal),
                };
                self.goal_active = goal
                    .as_ref()
                    .is_some_and(|goal| goal.status == crate::model::GoalStatus::Active);
                sink.send(Event::AgentUpdate(AgentUpdate::Goal(goal)))?;
            }
            "plan" => sink.send(Event::AgentUpdate(AgentUpdate::Plan(plan(update))))?,
            "usage_update" => sink.send(Event::AgentUpdate(AgentUpdate::Usage(ContextUsage {
                used: update["used"].as_u64().unwrap_or_default(),
                size: update["size"].as_u64().unwrap_or_default(),
                cost: update["cost"]["amount"]
                    .as_f64()
                    .filter(|amount| amount.is_finite()),
                currency: clipped(update["cost"]["currency"].as_str().unwrap_or_default(), 16),
            })))?,
            "notice" => sink.send(Event::AgentUpdate(AgentUpdate::Notice(format!(
                "{} {}",
                clipped(update["title"].as_str().unwrap_or_default(), 240),
                clipped(update["description"].as_str().unwrap_or_default(), 1000)
            ))))?,
            _ => {}
        }
        Ok(())
    }

    fn child_metadata(&mut self, source: &str, update: &Value, sink: &Sink) -> Result<()> {
        let Some(id) = update["sessionId"]
            .as_str()
            .or_else(|| update["subagentSessionId"].as_str())
        else {
            return Ok(());
        };
        if id.len() > 240 || (!self.children.contains_key(id) && self.children.len() >= 64) {
            return Ok(());
        }
        let parent = (Some(source) != self.root.as_deref()).then(|| clipped(source, 240));
        self.children.insert(id.into(), parent.clone());
        sink.send(Event::AgentUpdate(AgentUpdate::Child(ChildUpdate {
            id: id.into(),
            parent,
            name: update["title"]
                .as_str()
                .or_else(|| update["name"].as_str())
                .map(|text| clipped(text, 240)),
            task: update["description"]
                .as_str()
                .or_else(|| update["task"].as_str())
                .map(|text| clipped(text, 1000)),
            status: child_status(update).map(str::to_owned),
            event: None,
        })))
    }
}

fn text(update: &Value) -> Option<String> {
    update
        .pointer("/content/text")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn commands(update: &Value) -> Vec<SlashCommand> {
    update["availableCommands"]
        .as_array()
        .into_iter()
        .flatten()
        .take(40)
        .filter_map(|command| {
            let name = command["name"].as_str()?;
            if name.is_empty() || name.len() > 128 || name.chars().any(char::is_whitespace) {
                return None;
            }
            Some(SlashCommand {
                name: name.into(),
                description: clipped(command["description"].as_str().unwrap_or_default(), 1000),
                hint: clipped(command["input"]["hint"].as_str().unwrap_or_default(), 240),
            })
        })
        .collect()
}

fn plan(update: &Value) -> Vec<PlanItem> {
    update["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .take(32)
        .filter_map(|entry| {
            Some(PlanItem {
                content: clipped(entry["content"].as_str()?, 1000),
                status: match entry["status"].as_str() {
                    Some("completed") => "completed",
                    Some("in_progress") => "in_progress",
                    _ => "pending",
                }
                .into(),
            })
        })
        .collect()
}

fn child_status(update: &Value) -> Option<&'static str> {
    let state = update["state"]
        .as_str()
        .or_else(|| update["state"]["state"].as_str());
    match state {
        Some("running") => Some("running"),
        Some("requires_action") => Some("waiting"),
        Some("completed") => Some("done"),
        Some("cancelled") => Some("cancelled"),
        Some("failed") => Some("failed"),
        Some("disconnected") => Some("disconnected"),
        Some("idle") => Some(match update["state"]["stopReason"].as_str() {
            Some("cancelled") => "cancelled",
            Some("error") => "failed",
            _ => "done",
        }),
        Some(_) => Some("unknown"),
        None if update["sessionUpdate"] == "subagent_spawned" => Some("running"),
        None => None,
    }
}

fn clipped(text: &str, limit: usize) -> String {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}
