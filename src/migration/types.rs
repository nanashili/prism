use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OriginalProject {
    pub id: String,
    pub name: String,
    pub original: PathBuf,
    pub arch: String,
    pub kind: String,
    pub opened_at: i64,
}

#[derive(Default, Serialize)]
pub(super) struct Report {
    pub snapshot: PathBuf,
    pub projects_added: usize,
    pub chats_added: usize,
    pub messages_added: usize,
    pub functions_added: usize,
    pub views_added: usize,
    pub skipped_existing: usize,
    pub warnings: Vec<String>,
}
