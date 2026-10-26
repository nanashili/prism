use super::archive::Archive;
use crate::error::{Result, failure};
use crate::model::Project;
use crate::settings::support_dir;
use serde::Deserialize;
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn open(
    path: &Path,
    cancelled: &AtomicBool,
    budget: u64,
    protected: &[&Path],
) -> Result<Project> {
    let mut archive = Archive::open(path)?;
    let mut target = load_binary(&mut archive, cancelled, budget, protected)?;
    let manifest = archive.manifest.clone();
    target.name = manifest.name;
    target.kind = manifest.kind;
    target.arch = manifest.arch;
    let mut project = Project {
        target,
        source_document: Some(path.canonicalize()?),
        ..Project::default()
    };

    load_metadata(&mut archive, &mut project)?;
    Ok(project)
}

fn load_metadata(archive: &mut Archive, project: &mut Project) -> Result<()> {
    let entries = archive.manifest.entries.clone();
    let mut json_bytes = 0u64;
    for entry in &entries {
        if entry.name.ends_with(".json") {
            json_bytes = json_bytes
                .checked_add(entry.raw)
                .ok_or("The document metadata size overflowed")?;
            if json_bytes > 32 * 1024 * 1024 {
                return Err(failure("The document metadata exceeds 32 MiB"));
            }
            std::sync::Arc::make_mut(&mut project.imported).insert(
                entry.name.clone(),
                std::sync::Arc::new(archive.json(&entry.name)?),
            );
        } else if entry.name != "binary" {
            return Err(failure(format!(
                "Cannot preserve the document entry {} yet",
                entry.name
            )));
        }
    }
    if let Some(state) = project.imported.get("rust-project.json") {
        let restored: Project = Project::deserialize(state.as_ref())?;
        project.target.original_path = restored.target.original_path.or(Some(restored.target.path));
        project.renamed = restored.renamed;
        project.notes = restored.notes;
        project.bookmarks = restored.bookmarks;
        project.patches = restored.patches;
    } else if let Some(state) = project.imported.get("project.json") {
        if let Some(names) = state.get("names") {
            project.renamed = std::collections::BTreeMap::deserialize(names)?;
        }
        if let Some(notes) = state.get("notes") {
            project.notes = std::collections::BTreeMap::deserialize(notes)?;
        }
    }
    Ok(())
}

fn load_binary(
    archive: &mut Archive,
    cancelled: &AtomicBool,
    budget: u64,
    protected: &[&Path],
) -> Result<crate::model::Target> {
    let manifest = archive.manifest.clone();
    if manifest.binary.len() != 64 || !manifest.binary.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(failure("The document has an invalid binary identity"));
    }
    let binary_entry = manifest
        .entries
        .iter()
        .find(|entry| entry.name == "binary")
        .ok_or("The document has no binary entry")?;
    if binary_entry.sha256 != manifest.binary {
        return Err(failure(
            "The document records conflicting binary identities",
        ));
    }
    let root = support_dir().join("documents").join(&manifest.binary);
    let binary = root.join("binary");
    let cached = if binary.is_file() {
        Some(super::files::inspect(&binary, cancelled)?)
            .filter(|target| target.sha256 == manifest.binary)
    } else {
        None
    };
    let mut protected = protected.to_vec();
    if cached.is_some() {
        protected.push(&binary);
    }
    crate::storage::artifacts::reserve(
        if cached.is_some() {
            0
        } else {
            binary_entry.raw
        },
        budget,
        &protected,
        cancelled,
    )?;
    if cached.is_none() {
        crate::storage::atomic::write(&binary, |file| archive.extract("binary", file, budget))?;
    }
    crate::storage::artifacts::touch(&root)?;
    let target = match cached {
        Some(target) => target,
        None => super::files::inspect(&binary, cancelled)?,
    };
    if target.sha256 != manifest.binary {
        return Err(failure(
            "The saved binary identity does not match its contents",
        ));
    }
    Ok(target)
}
