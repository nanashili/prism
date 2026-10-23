use super::chat_activity::{OriginalMessage, convert};
use super::types::Report;
use crate::error::Result;
use crate::project::archive::Archive;
use crate::storage::Store;
use serde::Deserialize;
use std::collections::BTreeMap;
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
    let mut projects = ProjectLookup::default();
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
        insert(root, store, thread, report, &mut projects)?;
    }
    import_documents(root, store, report, &mut projects)
}

fn import_documents(
    root: &Path,
    store: &Store,
    report: &mut Report,
    projects: &mut ProjectLookup,
) -> Result<()> {
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
        import_archived_threads(&mut temporary, root, store, report, projects)?;
    }
    Ok(())
}

fn import_archived_threads(
    temporary: &mut File,
    root: &Path,
    store: &Store,
    report: &mut Report,
    projects: &mut ProjectLookup,
) -> Result<()> {
    // Validate the entire private, verified snapshot before any writes, just as
    // Vec<Thread> deserialization did. Each pass retains only one thread.
    temporary.rewind()?;
    crate::protocol::json_array::parse_reader(BufReader::new(&mut *temporary), |_: Thread| Ok(()))?;
    temporary.rewind()?;
    crate::protocol::json_array::parse_reader(BufReader::new(temporary), |thread| {
        insert(root, store, thread, report, projects)
    })
}

fn insert(
    root: &Path,
    store: &Store,
    thread: Thread,
    report: &mut Report,
    projects: &mut ProjectLookup,
) -> Result<()> {
    let project = projects.resolve(root, thread.project, thread.target.as_deref())?;
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

#[derive(Default)]
struct ProjectLookup {
    by_target: Option<BTreeMap<String, String>>,
}

impl ProjectLookup {
    fn resolve(
        &mut self,
        root: &Path,
        project: Option<String>,
        target: Option<&str>,
    ) -> Result<Option<String>> {
        if project.is_some() {
            return Ok(project);
        }
        // Import snapshots are immutable. Load lazily so chats with an explicit
        // project keep working without an index, and reuse it across both sources.
        if self.by_target.is_none() {
            let mut by_target = BTreeMap::new();
            for project in super::projects::index(root)? {
                if let Some(path) = project.original.to_str() {
                    // Preserve the previous first-match behavior for duplicate paths.
                    by_target.entry(path.to_owned()).or_insert(project.id);
                }
            }
            self.by_target = Some(by_target);
        }
        Ok(target.and_then(|target| self.by_target.as_ref()?.get(target).cloned()))
    }
}

#[cfg(test)]
mod tests;
