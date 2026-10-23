mod analysis;
mod assets;
mod chat_activity;
mod chats;
mod database_backup;
mod projects;
mod snapshot;
mod types;

use crate::error::{Result, failure};
use crate::settings::support_dir;
use crate::storage::Store;
pub(crate) use snapshot::lock as lock_data;
use std::ffi::OsString;

pub(crate) fn run(arguments: impl Iterator<Item = OsString>) -> Result<()> {
    let arguments: Vec<_> = arguments.collect();
    if arguments.len() > 1 {
        return Err(failure("Usage: prism import-upstream [snapshot-folder]"));
    }
    let destination = support_dir();
    let _lock = snapshot::lock(&destination)?;
    snapshot::ensure_closed(&destination)?;
    let root = match arguments.first() {
        Some(path) => std::path::PathBuf::from(path),
        None => snapshot::create(&destination)?,
    };
    let root = root.canonicalize()?;
    if !root.starts_with(destination.canonicalize()?.join("imports")) {
        return Err(failure(
            "Use a snapshot inside this Rust profile's imports folder",
        ));
    }
    if !root.join("snapshot-complete").is_file() {
        return Err(failure("Use a completed Prism import snapshot"));
    }
    snapshot::backup_native(&destination, &root)?;
    database_backup::compress(&root.join("support/reconstruction.sqlite"))?;

    let mut report = types::Report::default();
    assets::import(&root, &destination, &mut report)?;
    let store = Store::open(&destination.join("prism.db"))?;
    store.prepare_upstream_import()?;
    projects::import(&root, &store, &mut report)?;
    chats::import(&root, &store, &mut report)?;
    analysis::import(&root, &store, &mut report)?;
    store.verify_import()?;

    report.snapshot = root.clone();
    let path = if root.join("import-report.json").exists() {
        root.join("last-run-report.json")
    } else {
        root.join("import-report.json")
    };
    crate::storage::atomic::write(&path, |file| {
        serde_json::to_writer_pretty(file, &report)?;
        Ok(())
    })?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
