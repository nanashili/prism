use super::types::Report;
use crate::error::Result;
use crate::project::archive::Archive;
use crate::storage::Store;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub(super) fn import(root: &Path, store: &Store, report: &mut Report) -> Result<()> {
    for project in super::projects::index(root)? {
        let persistent = root.join("support/analysis").join(&project.id);
        if persistent.is_dir() {
            directory(&persistent, &persistent, &project.id, store, report)?;
        }
        let cached = root.join("cache-analysis").join(&project.id);
        if cached.is_dir() {
            directory(&cached, &cached, &project.id, store, report)?;
        }
        let path = root.join("documents").join(format!("{}.prism", project.id));
        if !path.is_file() {
            continue;
        }
        let value = Archive::open(&path)?.json("cache.json")?;
        let entries: BTreeMap<String, String> = serde_json::from_value(value)?;
        for (name, text) in entries {
            entry(&project.id, &name, &text, store, report)?;
        }
    }
    Ok(())
}

fn directory(
    root: &Path,
    path: &Path,
    project: &str,
    store: &Store,
    report: &mut Report,
) -> Result<()> {
    for item in std::fs::read_dir(path)? {
        let item = item?;
        if item.file_type()?.is_dir() {
            directory(root, &item.path(), project, store, report)?;
        } else if item.file_type()?.is_file() {
            let path = item.path();
            let relative = path.strip_prefix(root)?.to_string_lossy();
            let name = relative.strip_suffix(".zst").unwrap_or(&relative);
            let text = if relative.ends_with(".zst") {
                let mut decoder = zstd::stream::read::Decoder::new(File::open(&path)?)?;
                decoder.window_log_max(25)?;
                read_text(decoder)?
            } else {
                read_text(BufReader::new(File::open(&path)?))?
            };
            entry(project, name, &text, store, report)?;
        }
    }
    Ok(())
}

fn read_text(reader: impl Read) -> Result<String> {
    let mut text = String::new();
    reader
        .take(32 * 1024 * 1024 + 1)
        .read_to_string(&mut text)?;
    if text.len() > 32 * 1024 * 1024 {
        return Err(crate::error::failure(
            "An analysis cache entry exceeds 32 MiB",
        ));
    }
    Ok(text)
}

fn entry(project: &str, name: &str, text: &str, store: &Store, report: &mut Report) -> Result<()> {
    if name == "ghidra/procedures.json" {
        report.functions_added += store.import_procedures(project, text)?;
        return Ok(());
    }
    let view = if let Some(address) = name
        .strip_prefix("ghidra/pseudocode/")
        .and_then(|name| name.strip_suffix(".c"))
    {
        Some((address, "Pseudocode"))
    } else if let Some(address) = name
        .strip_prefix("ghidra/assembly/")
        .and_then(|name| name.strip_suffix(".s"))
    {
        Some((address, "Assembly"))
    } else if let Some(address) = name
        .strip_prefix("ghidra/dossier/")
        .and_then(|name| name.strip_suffix(".json"))
    {
        Some((address, "Dossier"))
    } else if name == "ghidra/segments.json" {
        Some(("", "Segments"))
    } else if name == "ghidra/strings.json" {
        Some(("", "Strings"))
    } else {
        None
    };
    if let Some((address, kind)) = view
        && store.import_view(project, address, kind, text)?
    {
        report.views_added += 1;
    }
    Ok(())
}
