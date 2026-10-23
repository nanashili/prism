use serde_json::Value;
use std::time::Duration;

/// Bounded admission classes shared by the helper and its window bridge.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CallClass {
    Ordinary,
    Question,
    Control,
    Snapshot,
}

impl CallClass {
    pub fn capacity(self) -> usize {
        match self {
            Self::Ordinary => 4,
            Self::Control | Self::Snapshot => 2,
            Self::Question => 8,
        }
    }

    pub fn byte_budget(self) -> usize {
        match self {
            Self::Ordinary => 16 * 1024 * 1024,
            Self::Question => 1024 * 1024,
            Self::Control | Self::Snapshot => 1024 * 1024,
        }
    }

    pub fn timeout(self) -> Duration {
        Duration::from_secs(if self == Self::Question { 3600 } else { 300 })
    }
}

pub fn classify_call(tool: &str, arguments: &Value) -> CallClass {
    if tool == super::question::TOOL {
        return CallClass::Question;
    }
    let saved_state = tool == "decompile_batch"
        && matches!(
            arguments["action"].as_str(),
            Some("list" | "status" | "results")
        );
    let window_state = matches!(tool, "get_state" | "current_view" | "list_marks");
    if saved_state || window_state {
        CallClass::Snapshot
    } else if durable_control(tool, arguments) {
        CallClass::Control
    } else {
        CallClass::Ordinary
    }
}

pub fn durable_control(tool: &str, arguments: &Value) -> bool {
    tool == "decompile_batch"
        && matches!(
            arguments["action"].as_str(),
            Some("pause" | "cancel" | "retry_failed")
        )
}
