use crate::error::{Result, failure};
use serde_json::{Value, json};

pub(super) enum ToolReply {
    Success(Value),
    Failure(ReaFailure),
}

pub(super) struct ReaFailure(Value);

impl ReaFailure {
    pub fn session_unavailable(&self) -> bool {
        if self.0["code"].as_str() == Some("cleanup_incomplete") {
            return true;
        }
        matches!(
            self.0
                .pointer("/details/diagnostics/failure_kind")
                .and_then(Value::as_str),
            Some("cancelled" | "timeout" | "start" | "process" | "protocol" | "analysis_timeout")
        )
    }

    pub fn resource_constraint(&self) -> bool {
        self.0["code"].as_str() == Some("resource_constraint")
    }

    pub fn retained_evidence(&self) -> Option<&str> {
        if !self.resource_constraint() {
            return None;
        }
        let direct = self.0.pointer("/details/evidence_reference");
        let nested = self
            .0
            .pointer("/details/reported_limits/evidence_reference");
        [direct, nested].into_iter().flatten().find_map(|value| {
            (value["kind"].as_str() == Some("retained-evidence"))
                .then(|| value["evidence_id"].as_str())
                .flatten()
        })
    }

    pub fn into_error(self) -> Box<dyn std::error::Error + Send + Sync> {
        failure(format!("REA: {}", self.0))
    }
}

impl ToolReply {
    pub fn into_result(self) -> Result<Value> {
        match self {
            Self::Success(value) => Ok(value),
            Self::Failure(error) => Err(error.into_error()),
        }
    }
}

pub(super) fn parse(mut reply: Value) -> Result<ToolReply> {
    let is_error = reply["isError"] == true;
    let mut content = reply_content(&mut reply, is_error)?;
    if is_error {
        let error = content.get_mut("error").map(Value::take).unwrap_or(content);
        return Ok(ToolReply::Failure(ReaFailure(error)));
    }
    if let Some(result) = content.get_mut("result").filter(|value| value.is_object()) {
        return Ok(ToolReply::Success(result.take()));
    }
    Ok(ToolReply::Success(content))
}

fn reply_content(reply: &mut Value, is_error: bool) -> Result<Value> {
    if let Some(content) = reply.get_mut("structuredContent") {
        return Ok(content.take());
    }

    let blocks = reply["content"].as_array().map(Vec::as_slice);
    let text = blocks
        .into_iter()
        .flatten()
        .filter_map(|block| block["text"].as_str())
        .collect::<String>();
    match serde_json::from_str(&text) {
        Ok(content) => Ok(content),
        Err(_) if is_error && plain_text_error(blocks) => Ok(json!({
            "code":"mcp_tool_error",
            "message":text,
            "details":{"diagnostics":{"failure_kind":"protocol"}}
        })),
        Err(error) => Err(failure(format!("Invalid REA response: {error}"))),
    }
}

fn plain_text_error(blocks: Option<&[Value]>) -> bool {
    // SDK output validation can fail after execution; its untyped error leaves
    // the prepared session's completion state unknown and must retire it.
    blocks.is_some_and(|blocks| {
        !blocks.is_empty()
            && blocks
                .iter()
                .all(|block| block["type"].as_str() == Some("text") && block["text"].is_string())
            && blocks.iter().any(|block| {
                block["text"]
                    .as_str()
                    .is_some_and(|text| !text.trim().is_empty())
            })
    })
}

#[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
pub(super) fn normalized(mut value: Value) -> Value {
    match value.get_mut("normalized_result") {
        Some(result) => result.take(),
        None => value,
    }
}
