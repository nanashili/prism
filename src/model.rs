mod activity;
mod agent_updates;
mod approval;
mod attachment;
mod goal;
mod json_preview;
mod queued_prompt;
mod turn_progress;

pub(crate) use activity::{Activity, Block, Outcome, ToolStep, ToolUpdate, now_ms};
pub(crate) use agent_updates::{
    AgentUpdate, ChildEvent, ChildUpdate, ContextUsage, PlanItem, SlashCommand,
};
pub(crate) use approval::Approval;
pub(crate) use attachment::Attachment;
pub(crate) use goal::{Goal, GoalStatus, GoalUpdate};
pub(crate) use json_preview::json_preview;
pub(crate) use queued_prompt::QueuedPrompt;
pub(crate) use turn_progress::{Child, TurnProgress};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Procedure {
    pub address: String,
    pub name: String,
    #[serde(default)]
    pub kind: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Target {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_path: Option<PathBuf>,
    pub name: String,
    pub sha256: String,
    pub kind: String,
    pub arch: String,
    pub size: u64,
}

impl Target {
    pub fn icon_path(&self) -> &std::path::Path {
        self.original_path.as_deref().unwrap_or(&self.path)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Screen {
    #[default]
    Home,
    Workbench,
    Settings,
    Setup,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum View {
    #[default]
    Pseudocode,
    Assembly,
    Graph,
    References,
    Strings,
    Imports,
    Segments,
    Hex,
    Resources,
    Notes,
    Diff,
    Debug,
    Reconstruct,
}

impl View {
    pub const ALL: [Self; 13] = [
        Self::Pseudocode,
        Self::Assembly,
        Self::Graph,
        Self::References,
        Self::Strings,
        Self::Imports,
        Self::Segments,
        Self::Hex,
        Self::Resources,
        Self::Notes,
        Self::Diff,
        Self::Debug,
        Self::Reconstruct,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pseudocode => "Pseudocode",
            Self::Assembly => "Assembly",
            Self::Graph => "Graph",
            Self::References => "References",
            Self::Strings => "Strings",
            Self::Imports => "Imports",
            Self::Segments => "Segments",
            Self::Hex => "Hex",
            Self::Resources => "Resources",
            Self::Notes => "Notes",
            Self::Diff => "Diff",
            Self::Debug => "Debugger",
            Self::Reconstruct => "Reconstruct project",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Message {
    #[serde(skip)]
    pub id: Option<i64>,
    pub role: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Project {
    pub target: Target,
    #[serde(default)]
    pub source_document: Option<PathBuf>,
    pub renamed: BTreeMap<String, String>,
    pub notes: BTreeMap<String, String>,
    pub bookmarks: Vec<String>,
    #[serde(default)]
    pub patches: BTreeMap<u64, Vec<u8>>,
    #[serde(default)]
    pub imported: std::sync::Arc<BTreeMap<String, serde_json::Value>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Recent {
    pub opened: i64,
    pub chats: i64,
    pub id: String,
    pub title: String,
    pub path: PathBuf,
    pub original: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct ChatSummary {
    pub id: i64,
    pub title: String,
    pub updated: i64,
    pub project: Option<String>,
    pub pinned: bool,
    pub archived: bool,
}

pub(crate) use goal::ConnectionState;
