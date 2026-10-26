use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Graph {
    pub version: u32,
    pub id: String,
    pub project: String,
    pub objective: String,
    pub revision: u32,
    pub paused: bool,
    pub budget: Budget,
    pub usage: Usage,
    pub tasks: Vec<Task>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Budget {
    pub steps: u32,
    pub functions: u32,
    pub depth: u32,
    pub evidence_bytes: u32,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            steps: 48,
            functions: 24,
            depth: 6,
            evidence_bytes: 256 * 1024,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Usage {
    pub steps: u32,
    pub functions: u32,
    pub evidence_bytes: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Task {
    pub id: u32,
    pub title: String,
    pub kind: Kind,
    pub dependencies: Vec<u32>,
    pub depth: u32,
    pub attempts: u32,
    pub state: TaskState,
    pub findings: Vec<Finding>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Finding {
    pub attempt: u32,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Kind {
    Survey,
    Locate,
    Inspect {
        address: String,
    },
    Resolve {
        from: String,
        to: Option<String>,
        question: String,
        evidence: String,
    },
    Synthesize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum TaskState {
    Pending,
    Running,
    Complete,
    Blocked { reason: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    Active,
    Paused,
    Blocked,
    Limited,
    Complete,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Report {
    pub findings: String,
    #[serde(default)]
    pub functions: Vec<String>,
    #[serde(default)]
    pub unresolved: Vec<Relationship>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Relationship {
    pub from: String,
    pub to: Option<String>,
    pub question: String,
    pub evidence: String,
}
