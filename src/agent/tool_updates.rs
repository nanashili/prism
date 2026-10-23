use serde_json::Value;

pub(super) fn parse(update: &Value) -> Option<crate::model::ToolUpdate> {
    use crate::model::{Outcome, ToolUpdate};
    let id = update["toolCallId"].as_str()?.chars().take(240).collect();
    let output = update.get("rawOutput").map(preview).or_else(|| {
        let content = update["content"].as_array()?;
        joined_preview(content.iter().filter_map(|entry| {
            entry
                .pointer("/content/text")
                .and_then(Value::as_str)
                .or_else(|| entry["text"].as_str())
        }))
    });
    Some(ToolUpdate {
        id,
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

fn joined_preview<'a>(texts: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut output = String::new();
    for (index, text) in texts.take(16).enumerate() {
        for part in [if index == 0 { "" } else { "\n\n" }, text] {
            let remaining = (16 * 1024usize).saturating_sub(output.len());
            if part.len() <= remaining {
                output.push_str(part);
                continue;
            }
            let mut end = remaining;
            while !part.is_char_boundary(end) {
                end -= 1;
            }
            output.push_str(&part[..end]);
            output.push_str("\n[Preview shortened]");
            return Some(output);
        }
    }
    (!output.is_empty()).then_some(output)
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

#[cfg(test)]
mod tests;
