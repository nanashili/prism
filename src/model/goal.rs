use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Goal {
    pub objective: String,
    pub status: GoalStatus,
    pub time_used_seconds: f64,
    pub created_at: f64,
    pub updated_at: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GoalStatus {
    Active,
    Paused,
    Blocked,
    Complete,
    Limited,
}

impl Goal {
    /// Missing or malformed metadata leaves the goal alone; explicit null clears it.
    pub fn from_metadata(meta: &Value) -> Option<Option<Self>> {
        let value = meta.pointer("/jetbrains/air/goal")?;
        if value.is_null() {
            return Some(None);
        }
        let mut goal: Self = serde_json::from_value(value.clone()).ok()?;
        if goal.objective.trim().is_empty()
            || goal.objective.chars().count() > 4000
            || [goal.time_used_seconds, goal.created_at, goal.updated_at]
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return None;
        }
        goal.objective = goal.objective.trim().into();
        Some(Some(goal))
    }

    pub fn elapsed(&self, connected: bool) -> u64 {
        let since_update = if connected && self.status == GoalStatus::Active {
            ((super::now_ms() as f64 - self.updated_at) / 1000.0).max(0.0)
        } else {
            0.0
        };
        (self.time_used_seconds + since_update) as u64
    }
}

impl GoalStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "Pursuing goal",
            Self::Paused => "Goal paused",
            Self::Blocked => "Goal blocked",
            Self::Complete => "Goal complete",
            Self::Limited => "Goal limit reached",
        }
    }
}
