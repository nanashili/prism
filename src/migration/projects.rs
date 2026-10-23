use super::types::{OriginalProject, Report};
use crate::error::{Result, failure};
use crate::model::{Project, Target};
use crate::project::{archive::Archive, files};
use crate::storage::Store;
use serde_json::{Value, json};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

pub(super) fn validate_id(id: &str) -> Result<()> {
    if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(failure(
            "An original project has an invalid SHA-256 identity",
        ));
    }
    Ok(())
}

pub(super) fn index(root: &Path) -> Result<Vec<OriginalProject>> {
    Ok(serde_json::from_reader(BufReader::new(File::open(
        root.join("support/projects/index.json"),
    )?))?)
}

pub(super) fn import(root: &Path, store: &Store, report: &mut Report) -> Result<()> {
    for original in index(root)? {
        validate_id(&original.id)?;
        match store.project(&original.id) {
            Ok(_) => {
                report.skipped_existing += 1;
                continue;
            }
            Err(error)
                if error
                    .downcast_ref::<rusqlite::Error>()
                    .is_some_and(|error| matches!(error, rusqlite::Error::QueryReturnedNoRows)) => {
            }
            Err(error) => return Err(error),
        }
        let binary = binary(root, &original, report)?;
        let state = project_state(root, &original.id)?;
        let opened = original.opened_at;
        let project = convert(original, binary, state, root)?;
        if store.import_project(&project, opened)? {
            report.projects_added += 1;
        }
    }
    Ok(())
}

fn convert(
    original: OriginalProject,
    binary: PathBuf,
    state: Value,
    root: &Path,
) -> Result<Project> {
    let mut project = Project {
        target: Target {
            size: if binary.is_file() {
                std::fs::metadata(&binary)?.len()
            } else {
                0
            },
            path: binary,
            original_path: Some(original.original),
            name: original.name,
            sha256: original.id,
            kind: original.kind,
            arch: original.arch,
        },
        renamed: serde_json::from_value(state.get("names").cloned().unwrap_or(json!({})))?,
        notes: serde_json::from_value(state.get("notes").cloned().unwrap_or(json!({})))?,
        ..Project::default()
    };
    if let Some(bookmarks) = state.get("bookmarks").and_then(Value::as_array) {
        project.bookmarks = bookmarks
            .iter()
            .filter_map(|entry| entry.get("addr").and_then(Value::as_str).map(str::to_owned))
            .collect();
    }
    let imported = std::sync::Arc::make_mut(&mut project.imported);
    imported.insert("project.json".into(), state);
    imported.insert("upstream-snapshot".into(), json!(root));
    Ok(project)
}

fn project_state(root: &Path, id: &str) -> Result<Value> {
    let current = root.join("support/projects").join(format!("{id}.json"));
    if current.is_file() {
        return Ok(serde_json::from_reader(BufReader::new(File::open(
            current,
        )?))?);
    }
    let document = root.join("documents").join(format!("{id}.prism"));
    if document.is_file() {
        return Archive::open(&document)?.json("project.json");
    }
    Ok(json!({}))
}

fn binary(root: &Path, original: &OriginalProject, report: &mut Report) -> Result<PathBuf> {
    let folder = root.join("binaries").join(&original.id);
    std::fs::create_dir_all(&folder)?;
    let destination = folder.join("binary");
    let cancelled = AtomicBool::new(false);
    if destination.is_file() && files::hash(&destination, &cancelled)? == original.id {
        return Ok(destination);
    }
    let document = root
        .join("documents")
        .join(format!("{}.prism", original.id));
    if document.is_file() {
        let mut archive = Archive::open(&document)?;
        if archive.manifest.binary != original.id {
            return Err(failure("The saved binary belongs to a different project"));
        }
        crate::storage::atomic::write(&destination, |output| {
            archive.extract("binary", output, 8 * 1024 * 1024 * 1024)
        })?;
    } else if original.original.exists() {
        copy_executable(original, &destination)?;
    } else {
        report.warnings.push(format!(
            "{}: original executable is missing; its records were preserved",
            original.name
        ));
        return Ok(original.original.clone());
    }
    if files::hash(&destination, &cancelled)? != original.id {
        return Err(failure(format!(
            "{} has changed since its original analysis",
            original.name
        )));
    }
    Ok(destination)
}

fn copy_executable(original: &OriginalProject, destination: &Path) -> Result<()> {
    let executable = files::executable(&original.original)?;
    let cancelled = AtomicBool::new(false);
    if files::hash(&executable, &cancelled)? == original.id {
        return super::snapshot::copy_file(&executable, destination);
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("/usr/bin/lipo")
            .arg(&executable)
            .arg("-thin")
            .arg(&original.arch)
            .arg("-output")
            .arg(destination)
            .output()?;
        if output.status.success() {
            return Ok(());
        }
        Err(failure(format!(
            "Could not recover {}: {}",
            original.name,
            String::from_utf8_lossy(&output.stderr)
        )))
    }
    #[cfg(not(target_os = "macos"))]
    Err(failure(
        "Recovering the original architecture requires its saved Prism document",
    ))
}

#[cfg(test)]
mod tests;
