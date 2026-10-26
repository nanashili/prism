use crate::error::Result;
use crate::model::{Message, Project, View};
use crate::settings::Settings;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::mpsc;

pub(crate) enum AnalysisExport {
    Function { view: View, address: String },
    AllPseudocode,
    ObjectiveC,
}

pub(crate) enum ViewJob {
    Content {
        view: View,
        address: String,
        offset: u64,
    },
    Tool {
        name: String,
        args: Value,
    },
    Diff(PathBuf),
    Debug(crate::debug::Action),
}

pub(crate) enum Job {
    Open(PathBuf),
    Index,
    StartInvestigation {
        project: String,
        objective: String,
        budget: crate::investigation::Budget,
    },
    Sidebar {
        project: String,
        generation: u64,
        query: crate::storage::navigation::Query,
    },
    Resources {
        project: String,
        generation: u64,
        parent: String,
        offset: usize,
    },
    View {
        request: u64,
        job: ViewJob,
    },
    CopyCode {
        view: View,
        address: String,
    },
    Save {
        path: PathBuf,
        project: Project,
        messages: Vec<Message>,
    },
    Patch {
        path: PathBuf,
        project: Project,
    },
    Export {
        path: PathBuf,
        text: String,
    },
    ExportAnalysis {
        path: PathBuf,
        kind: AnalysisExport,
    },
    Configure(Box<Settings>),
    Reconstruct {
        tool: String,
        args: Value,
    },
    Remote {
        tool: String,
        args: Value,
        answer: mpsc::SyncSender<Result<Value>>,
        lifetime: Option<super::RequestLifetime>,
    },
}
