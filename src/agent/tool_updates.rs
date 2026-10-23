use serde_json::Value;

pub(super) fn parse(update: &Value) -> Option<crate::model::ToolUpdate> {
    use crate::model::{Outcome, ToolUpdate};
    let output = update.get("rawOutput").map(preview).or_else(|| {
        let content = update["content"].as_array()?;
        let text = content
            .iter()
            .filter_map(|entry| {
                entry
                    .pointer("/content/text")
                    .and_then(Value::as_str)
                    .or_else(|| entry["text"].as_str())
            })
            .take(16)
            .collect::<Vec<_>>()
            .join("\n\n");
        (!text.is_empty()).then(|| clipped(&text))
    });
    Some(ToolUpdate {
        id: update["toolCallId"].as_str()?.chars().take(240).collect(),
        title: update["title"].as_str().map(clipped),
        status: update["status"].as_str().map(|status| match status {
            "completed" => Outcome::Done,
            "failed" => Outcome::Failed,
            _ => Outcome::Running,
        }),
        input: update.get("rawInput").map(preview),
        output,
    })
}

fn preview(value: &Value) -> String {
    crate::model::json_preview(value, 16 * 1024)
}

fn clipped(text: &str) -> String {
    let mut end = text.len().min(16 * 1024);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut result = text[..end].to_owned();
    if end < text.len() {
        result.push_str("\n[Preview shortened]");
    }
    result
}
