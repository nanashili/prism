use super::{archive::Archive, files, save};
use crate::error::Result;
use crate::model::Project;
use std::path::Path;
use std::sync::atomic::AtomicBool;

const INPUT: &[u8] = b"\x7fELF\x02\x01\x01\0sample program bytes";

fn sample_project(directory: &Path) -> Result<Project> {
    let path = directory.join("sample.bin");
    std::fs::write(&path, INPUT)?;
    let mut project = Project {
        target: files::inspect(&path, &AtomicBool::new(false))?,
        ..Project::default()
    };
    project.renamed.insert("0x1000".into(), "entry".into());
    project
        .notes
        .insert("0x1000".into(), "Validate input".into());
    Ok(project)
}

#[test]
fn saved_prism_archive_preserves_binary_and_project_metadata() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let project = sample_project(directory.path())?;
    let destination = directory.path().join("investigation.prism");

    save(&destination, &project, &[])?;

    let bytes = std::fs::read(&destination)?;
    assert_eq!(&bytes[..8], b"PRISM\0\0\x01");
    let mut archive = Archive::open(&destination)?;
    let mut binary = Vec::new();
    archive.extract("binary", &mut binary, 1024)?;
    assert_eq!(binary, INPUT);
    assert_eq!(std::fs::read(&project.target.path)?, INPUT);

    let restored: Project = serde_json::from_value(archive.json("rust-project.json")?)?;
    assert_eq!(restored.target.sha256, project.target.sha256);
    assert_eq!(
        restored.renamed.get("0x1000").map(String::as_str),
        Some("entry")
    );
    assert_eq!(
        restored.notes.get("0x1000").map(String::as_str),
        Some("Validate input")
    );
    Ok(())
}

#[test]
fn saving_changed_input_preserves_the_existing_archive_and_input() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let project = sample_project(directory.path())?;
    let destination = directory.path().join("investigation.prism");
    save(&destination, &project, &[])?;
    let saved = std::fs::read(&destination)?;

    let changed = b"changed program bytes";
    std::fs::write(&project.target.path, changed)?;
    let error = save(&destination, &project, &[]).expect_err("Changed input must be rejected");

    assert!(error.to_string().contains("The input changed"));
    assert_eq!(std::fs::read(&destination)?, saved);
    assert_eq!(std::fs::read(&project.target.path)?, changed);

    let error = save(&project.target.path, &project, &[])
        .expect_err("Saving over the input must be rejected");
    assert!(error.to_string().contains("preserve the original file"));
    assert_eq!(std::fs::read(&project.target.path)?, changed);
    Ok(())
}
