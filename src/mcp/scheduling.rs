use serde_json::Value;
use std::time::Duration;

/// Bounded admission classes shared by the helper and its window bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallClass {
    /// Analysis and other work that consumes a worker slot.
    Ordinary,
    /// Questions that may wait for user input.
    Question,
    /// Pause, cancellation, and retry requests.
    Control,
    /// Read-only window state or saved durable-job queries.
    Snapshot,
}

impl CallClass {
    #[must_use]
    /// Maximum concurrent requests admitted for this class.
    pub fn capacity(self) -> usize {
        match self {
            Self::Ordinary => 4,
            Self::Control | Self::Snapshot => 2,
            Self::Question => 8,
        }
    }

    #[must_use]
    /// Maximum aggregate request bytes retained by this class.
    pub fn byte_budget(self) -> usize {
        match self {
            Self::Ordinary => 16 * 1024 * 1024,
            Self::Question => 1024 * 1024,
            Self::Control | Self::Snapshot => 1024 * 1024,
        }
    }

    #[must_use]
    /// Deadline duration for one admitted request.
    pub fn timeout(self) -> Duration {
        Duration::from_secs(if self == Self::Question { 3600 } else { 300 })
    }
}

#[must_use]
/// Classifies a tool call for admission, memory limits, and timeout selection.
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

#[must_use]
/// Identifies durable-job control requests that bypass ordinary analysis admission.
pub fn durable_control(tool: &str, arguments: &Value) -> bool {
    tool == "decompile_batch"
        && matches!(
            arguments["action"].as_str(),
            Some("pause" | "cancel" | "retry_failed")
        )
}
