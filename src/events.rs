use crate::error::{Result, failure};
use crate::model::{Procedure, Project, View};
use serde_json::Value;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

pub(crate) enum Event {
    Chat {
        key: u64,
        event: Box<Event>,
    },
    Progress(String),
    Analysis(Option<AnalysisProgress>),
    InvestigationChanged {
        project: String,
        id: String,
    },
    InvestigationCreated(Box<crate::investigation::Graph>),
    Reconstruction {
        project: String,
        status: Box<Value>,
        start_agent: bool,
    },
    Opened(Box<Project>, bool),
    Functions {
        rows: Vec<Procedure>,
        offset: usize,
    },
    Sidebar {
        project: String,
        generation: u64,
        parent: String,
        offset: usize,
        page: std::result::Result<crate::storage::navigation::Page, String>,
    },
    Resources {
        project: String,
        generation: u64,
        parent: String,
        offset: usize,
        page: std::result::Result<crate::storage::resources::ResourcePage, String>,
    },
    Content {
        request: Option<u64>,
        view: View,
        address: String,
        text: String,
    },
    Clipboard(String),
    Saved(std::path::PathBuf),
    Exported(std::path::PathBuf),
    Finished(Result<String>),
    AgentText(String),
    AgentThought(String),
    AgentTool(crate::model::ToolUpdate),
    AgentReady(Value),
    AgentUpdate(crate::model::AgentUpdate),
    AgentFinished {
        result: Result<()>,
        stopped: bool,
    },
    Permission {
        request: crate::model::Approval,
        answer: SyncSender<Option<String>>,
        active: std::sync::Arc<std::sync::atomic::AtomicBool>,
    },
    Question {
        question: prism::mcp::question::Question,
        answer: SyncSender<Result<Value>>,
        active: std::sync::Arc<std::sync::atomic::AtomicBool>,
    },
    Navigate {
        tool: String,
        args: Value,
        answer: SyncSender<Result<Value>>,
        lifetime: crate::worker::RequestLifetime,
    },
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AnalysisProgress {
    pub label: String,
    pub completed: u64,
    pub total: Option<u64>,
}

#[derive(Clone)]
pub(crate) struct Sink {
    sender: SyncSender<Event>,
    context: eframe::egui::Context,
    chat: Option<u64>,
}

impl Sink {
    pub fn repaint(&self) {
        self.context.request_repaint();
    }

    pub fn analysis(&self, label: &str, completed: u64, total: Option<u64>) -> Result<()> {
        self.send(Event::Analysis(Some(AnalysisProgress {
            label: label.into(),
            completed,
            total,
        })))
    }

    pub fn channel(context: &eframe::egui::Context) -> (Self, Receiver<Event>) {
        let (sender, receiver) = sync_channel(32);
        (
            Self {
                sender,
                context: context.clone(),
                chat: None,
            },
            receiver,
        )
    }

    pub fn send(&self, event: Event) -> Result<()> {
        let event = match self.chat {
            Some(key) => Event::Chat {
                key,
                event: Box::new(event),
            },
            None => event,
        };
        self.context.request_repaint();
        self.sender
            .send(event)
            .map_err(|_| failure("The application has closed"))?;
        self.context.request_repaint();
        Ok(())
    }

    pub fn try_send(&self, event: Event) -> Result<Option<Event>> {
        let event = match self.chat {
            Some(key) => Event::Chat {
                key,
                event: Box::new(event),
            },
            None => event,
        };
        self.context.request_repaint();
        match self.sender.try_send(event) {
            Ok(()) => {
                self.context.request_repaint();
                Ok(None)
            }
            Err(std::sync::mpsc::TrySendError::Full(event)) => Ok(Some(match event {
                Event::Chat { event, .. } if self.chat.is_some() => *event,
                event => event,
            })),
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                Err(failure("The application has closed"))
            }
        }
    }

    pub fn for_chat(&self, key: u64) -> Self {
        Self {
            chat: Some(key),
            ..self.clone()
        }
    }
}
