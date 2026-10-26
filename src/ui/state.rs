use crate::model::{Message, Procedure, Project};
use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

#[derive(Default)]
pub(super) struct Workspace {
    pub project: Option<Project>,
    pub comparison: Option<std::sync::Arc<crate::comparison::Report>>,
    pub reviewed_matches: Vec<crate::comparison::Selection>,
    pub agent_closed: bool,
    pub split: bool,
    pub split_fraction: Option<f32>,
    pub panes: [super::view_pane::ViewPane; 2],
    pub focused_pane: usize,
    pub rendering_pane: Option<usize>,
    pub next_request: u64,
    pub next_pane_identity: u64,
    pub rows: Vec<Procedure>,
    pub navigation: super::bench_sidebar::Navigation,
    pub filter: String,
    pub offset: usize,
    pub document: Option<PathBuf>,
    pub undo: Vec<Project>,
}

impl std::ops::Deref for Workspace {
    type Target = super::view_pane::ViewPane;

    fn deref(&self) -> &Self::Target {
        // Rendering selects a pane temporarily; commands between frames use keyboard focus.
        &self.panes[self.rendering_pane.unwrap_or(self.focused_pane)]
    }
}

impl std::ops::DerefMut for Workspace {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.panes[self.rendering_pane.unwrap_or(self.focused_pane)]
    }
}

#[derive(Default)]
pub(super) struct Chat {
    pub id: Option<i64>,
    pub title: String,
    pub messages: Vec<Message>,
    pub transcript_ids: super::chat::transcript::MessageIdentities,
    pub history: super::chat_history::History,
    pub draft: String,
    pub attachments: Vec<crate::model::Attachment>,
    pub queue: std::collections::VecDeque<crate::model::QueuedPrompt>,
    pub mentions: Vec<super::composer::mentions::Mention>,
    pub busy: bool,
    pub status: String,
    pub permission: Option<Permission>,
    pub questions: super::chat::questions::Questions,
    pub options: serde_json::Value,
    pub composer_revision: u64,
    pub commands: Vec<crate::model::SlashCommand>,
    pub usage: Option<crate::model::ContextUsage>,
    pub goal: Option<crate::model::Goal>,
    pub connected: bool,
    pub picks: Vec<crate::agent::options::Pick>,
    pub models: crate::storage::ChatModels,
}

pub(super) struct Permission {
    pub request: crate::model::Approval,
    pub answer: SyncSender<Option<String>>,
    pub active: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Chat {
    pub fn trim_history(&mut self) {
        let mut bytes: usize = self.messages.iter().map(message_bytes).sum();
        let mut removed = 0;
        for message in &self.messages {
            if self.messages.len() - removed <= 40 && bytes <= 2 * 1024 * 1024 {
                break;
            }
            bytes -= message_bytes(message);
            removed += 1;
        }
        self.history.has_earlier |= self.messages[..removed]
            .iter()
            .any(|message| message.id.is_some());
        self.messages.drain(..removed);
        self.transcript_ids.remove_front(removed);
    }
}

fn message_bytes(message: &Message) -> usize {
    message.text.len()
        + message
            .activity
            .as_ref()
            .map_or(0, crate::model::Activity::retained_bytes)
}
