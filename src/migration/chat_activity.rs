use crate::model::{Activity, Block, Message, Outcome, ToolUpdate};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OriginalMessage {
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    blocks: Vec<OriginalBlock>,
    #[serde(default)]
    started_at: i64,
    #[serde(default)]
    ended_at: Option<i64>,
    #[serde(default)]
    outcome: String,
}

#[derive(Deserialize)]
struct OriginalBlock {
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    step: Option<OriginalTool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OriginalTool {
    id: String,
    tool: String,
    status: String,
    #[serde(default)]
    started_at: i64,
    #[serde(default)]
    ended_at: Option<i64>,
}

pub(super) fn convert(message: OriginalMessage, agent: &str) -> Option<Message> {
    if message.kind != "agent" {
        return (!message.text.is_empty()).then_some(Message {
            id: None,
            role: if message.kind == "user" {
                "user"
            } else {
                "assistant"
            }
            .into(),
            text: message.text,
            activity: None,
        });
    }
    let mut activity = Activity::start(agent);
    activity.started = message.started_at;
    activity.ended = message.ended_at;
    activity.outcome = outcome(&message.outcome);
    let mut text = String::new();
    let mut text_blocks = 0;
    for block in message.blocks {
        match block.kind.as_str() {
            "text" => {
                let start = text.len();
                if text_blocks > 0 {
                    text.push_str("\n\n");
                }
                text.push_str(&block.text);
                text_blocks += 1;
                activity.record_text(start, text.len());
            }
            "thought" => activity.record_thought(&block.text),
            "tool" => {
                if let Some(tool) = block.step {
                    restore_tool(&mut activity, tool);
                }
            }
            _ => {}
        }
    }
    // Existing imports matched text-bearing messages. Keep those identities stable.
    (!text.is_empty()).then_some(Message {
        id: None,
        role: "assistant".into(),
        text,
        activity: Some(activity),
    })
}

fn restore_tool(activity: &mut Activity, original: OriginalTool) {
    let id = original.id.clone();
    activity.update_tool(ToolUpdate {
        id: original.id,
        title: Some(original.tool),
        status: Some(outcome(&original.status)),
        input: None,
        output: Some("Tool details are retained in the original import snapshot.".into()),
    });
    if let Some(Block::Tool(tool)) = activity
        .blocks
        .iter_mut()
        .find(|block| matches!(block, Block::Tool(tool) if tool.id == id))
    {
        tool.started = original.started_at;
        tool.ended = original.ended_at.filter(|_| tool.started > 0);
    }
}

fn outcome(value: &str) -> Outcome {
    match value {
        "done" => Outcome::Done,
        "failed" => Outcome::Failed,
        "running" | "cancelled" => Outcome::Stopped,
        _ => Outcome::Done,
    }
}
