use super::chat_activity::{OriginalMessage, convert};
use super::types::Report;
use crate::error::Result;
use crate::project::archive::Archive;
use crate::storage::Store;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufReader, Seek};
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Thread {
    id: String,
    title: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    messages: Vec<OriginalMessage>,
    #[serde(default)]
    agent: String,
    updated_at: i64,
}

pub(super) fn import(root: &Path, store: &Store, report: &mut Report) -> Result<()> {
    let directory = root.join("support/threads");
    let mut paths = Vec::new();
    if directory.is_dir() {
        for entry in std::fs::read_dir(&directory)? {
            paths.push(entry?.path());
        }
    }
    paths.sort();
    for path in paths {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let thread: Thread = if name.ends_with(".json.zst") {
            let mut decoder = zstd::stream::read::Decoder::new(File::open(&path)?)?;
            decoder.window_log_max(27)?;
            serde_json::from_reader(BufReader::new(decoder))?
        } else if name.ends_with(".json") {
            if path.with_extension("json.zst").is_file() {
                continue;
            }
            serde_json::from_reader(BufReader::new(File::open(&path)?))?
        } else {
            continue;
        };
        insert(root, store, thread, report)?;
    }
    import_documents(root, store, report)
}

fn import_documents(root: &Path, store: &Store, report: &mut Report) -> Result<()> {
    let folder = root.join("documents");
    if !folder.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        if path
            .extension()
            .is_none_or(|extension| extension != "prism")
        {
            continue;
        }
        let mut archive = Archive::open(&path)?;
        let mut temporary = tempfile::tempfile()?;
        archive.extract("chats.json", &mut temporary, 1024 * 1024 * 1024)?;
        temporary.rewind()?;
        // Unknown tool payloads are skipped while parsing, rather than held in memory.
        let threads: Vec<Thread> = serde_json::from_reader(BufReader::new(temporary))?;
        for thread in threads {
            insert(root, store, thread, report)?;
        }
    }
    Ok(())
}

fn insert(root: &Path, store: &Store, thread: Thread, report: &mut Report) -> Result<()> {
    let project = match thread.project {
        Some(project) => Some(project),
        None => super::projects::index(root)?
            .into_iter()
            .find(|project| thread.target.as_deref() == project.original.to_str())
            .map(|project| project.id),
    };
    let messages: Vec<_> = thread
        .messages
        .into_iter()
        .filter_map(|message| convert(message, &thread.agent))
        .collect();
    if store.import_thread(
        &thread.id,
        project.as_deref(),
        &thread.title,
        thread.updated_at,
        &messages,
    )? {
        report.chats_added += 1;
        report.messages_added += messages.len();
    } else {
        report.skipped_existing += 1;
    }
    Ok(())
}
