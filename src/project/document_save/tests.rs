use super::*;
use serde_json::json;

#[test]
fn borrowed_metadata_matches_project_and_messages_without_imported_copies() -> Result<()> {
    let mut project = Project::default();
    project.target.path = "target.bin".into();
    project.target.original_path = Some("original.bin".into());
    project.source_document = Some("saved.prism".into());
    project.renamed.insert("0x10".into(), "renamed".into());
    project.notes.insert("0x10".into(), "new note 🦀".into());
    project.bookmarks.push("0x10".into());
    project.patches.insert(42, vec![0, 255, 10]);
    std::sync::Arc::make_mut(&mut project.imported).extend(
        [
            (
                "project.json".into(),
                json!({
                    "names":{"old":"old name"}, "notes":{"old":"old note"},
                    "unknown":{"nested":[1,true,null,"preserved"]}
                }),
            ),
            ("extra.json".into(), json!(["extra", 7])),
            ("rust-project.json".into(), json!({"stale":true})),
            ("rust-chat.json".into(), json!(["stale"])),
        ]
        .map(|(name, value)| (name, std::sync::Arc::new(value))),
    );
    let messages = [Message {
        id: Some(12),
        role: "assistant".into(),
        text: "response with \"quotes\"".into(),
        activity: None,
    }];
    let original = serde_json::to_value(&project)?;
    let parts = metadata(&project, &messages)?;
    let mut expected = original.clone();
    expected["imported"] = json!({});
    assert_eq!(serde_json::to_value(&parts["rust-project.json"])?, expected);
    assert_eq!(
        serde_json::to_value(&parts["rust-chat.json"])?,
        serde_json::to_value(&messages)?
    );
    let mut expected = project.imported["project.json"].as_ref().clone();
    expected["names"] = serde_json::to_value(&project.renamed)?;
    expected["notes"] = serde_json::to_value(&project.notes)?;
    assert_eq!(serde_json::to_value(&parts["project.json"])?, expected);
    assert_eq!(
        serde_json::to_value(&parts["extra.json"])?,
        *project.imported["extra.json"]
    );
    assert_eq!(serde_json::to_value(&project)?, original);
    Ok(())
}

#[test]
fn metadata_keeps_missing_invalid_and_entry_limit_behavior() -> Result<()> {
    let mut project = Project::default();
    assert_eq!(
        serde_json::to_value(&metadata(&project, &[])?["project.json"])?,
        json!({"names":{},"notes":{}})
    );
    std::sync::Arc::make_mut(&mut project.imported)
        .insert("project.json".into(), std::sync::Arc::new(Value::Null));
    assert!(metadata(&project, &[]).is_err());
    std::sync::Arc::make_mut(&mut project.imported).clear();
    for index in 0..253 {
        std::sync::Arc::make_mut(&mut project.imported).insert(
            format!("entry-{index}.json"),
            std::sync::Arc::new(Value::Null),
        );
    }
    assert!(metadata(&project, &[]).is_err());
    std::sync::Arc::make_mut(&mut project.imported).remove("entry-252.json");
    assert_eq!(metadata(&project, &[])?.len(), 255);
    Ok(())
}

// Kept only for the paired RSS workload: this is the materializing save path
// from cc2c4f6, adapted to borrow the equivalent per-entry Arc<Value> contents.
fn baseline_metadata<'a>(
    project: &'a Project,
    messages: &[Message],
) -> Result<BTreeMap<&'a str, std::borrow::Cow<'a, Value>>> {
    use std::borrow::Cow;
    let mut parts: BTreeMap<_, _> = project
        .imported
        .iter()
        .map(|(name, value)| (name.as_str(), Cow::Borrowed(value.as_ref())))
        .collect();
    let mut state = project.clone();
    state.imported = Default::default();
    parts.insert(
        "rust-project.json",
        Cow::Owned(serde_json::to_value(&state)?),
    );
    parts.insert(
        "rust-chat.json",
        Cow::Owned(serde_json::to_value(messages)?),
    );
    let mut upstream = parts
        .remove("project.json")
        .unwrap_or_else(|| Cow::Owned(json!({})));
    let properties = upstream
        .to_mut()
        .as_object_mut()
        .ok_or("Invalid project.json")?;
    properties.insert("names".into(), serde_json::to_value(&project.renamed)?);
    properties.insert("notes".into(), serde_json::to_value(&project.notes)?);
    parts.insert("project.json", upstream);
    if parts.len() >= 256 {
        return Err("The document has too many metadata entries".into());
    }
    Ok(parts)
}

fn baseline_save(path: &Path, project: &Project, messages: &[Message]) -> Result<()> {
    super::super::files::preserve_original(&project.target.path, path)?;
    let parent = path.parent().ok_or("Missing parent")?;
    std::fs::create_dir_all(parent)?;
    let mut payload = tempfile::tempfile_in(parent)?;
    let parts = baseline_metadata(project, messages)?;
    let entries = pack_payload(project, parts, &mut payload)?;
    let manifest = manifest(project, entries)?;
    payload.seek(SeekFrom::Start(0))?;
    crate::storage::atomic::write(path, |file| {
        file.write_all(MAGIC)?;
        file.write_all(&u32::try_from(manifest.len())?.to_le_bytes())?;
        file.write_all(&manifest)?;
        std::io::copy(&mut payload, file)?;
        Ok(())
    })
}

#[test]
fn saved_archive_matches_materialized_baseline_semantically() -> Result<()> {
    use sha2::{Digest, Sha256};

    let directory = tempfile::tempdir()?;
    let binary = directory.path().join("sample.bin");
    std::fs::write(&binary, b"original bytes")?;
    let mut project = Project::default();
    project.target.path = binary;
    project.target.sha256 = format!("{:x}", Sha256::digest(b"original bytes"));
    project.target.size = 14;
    project.renamed.insert("0x10".into(), "entry".into());
    project.notes.insert("0x10".into(), "note 🦀".into());
    project.patches.insert(2, vec![1, 2, 3]);
    project.patches.insert(10, vec![4, 5]);
    std::sync::Arc::make_mut(&mut project.imported).extend(
        [
            (
                "project.json".into(),
                json!({"names":{"old":"old"},"notes":null,"extra":[{"nested":"preserved"}]}),
            ),
            ("another.json".into(), json!(["unknown", true])),
        ]
        .map(|(name, value)| (name, std::sync::Arc::new(value))),
    );
    let mut activity = crate::model::Activity::start("test agent");
    activity.record_text(0, 8);
    let messages = [Message {
        id: Some(1),
        role: "assistant".into(),
        text: "response".into(),
        activity: Some(activity),
    }];
    let baseline = directory.path().join("baseline.prism");
    let candidate = directory.path().join("candidate.prism");
    baseline_save(&baseline, &project, &messages)?;
    save(&candidate, &project, &messages)?;
    let mut baseline = super::super::archive::Archive::open(&baseline)?;
    let mut candidate = super::super::archive::Archive::open(&candidate)?;
    let names: Vec<_> = baseline
        .manifest
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
    assert_eq!(
        names,
        candidate
            .manifest
            .entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<Vec<_>>()
    );
    for name in names {
        if name.ends_with(".json") {
            assert_eq!(baseline.json(&name)?, candidate.json(&name)?, "{name}");
        } else {
            let mut before = Vec::new();
            let mut after = Vec::new();
            baseline.extract(&name, &mut before, 1024)?;
            candidate.extract(&name, &mut after, 1024)?;
            assert_eq!(before, after);
        }
    }
    Ok(())
}

#[test]
#[ignore = "fresh-process RSS workload; requires PRISM_MEMORY_FIXTURE JSON object"]
fn memory_project_save() -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::hint::black_box;
    use std::io::BufReader;
    use std::time::Instant;

    let input = std::env::var_os("PRISM_MEMORY_FIXTURE").ok_or("Set PRISM_MEMORY_FIXTURE")?;
    let mode = std::env::var("PRISM_MEMORY_IMPL").unwrap_or_else(|_| "candidate".into());
    let imported: Value = serde_json::from_reader(BufReader::new(File::open(input)?))?;
    assert!(imported.is_object());
    let directory = tempfile::tempdir()?;
    let binary = directory.path().join("sample.bin");
    std::fs::write(&binary, b"benchmark executable")?;
    let mut project = Project::default();
    project.target.path = binary;
    project.target.sha256 = format!("{:x}", Sha256::digest(b"benchmark executable"));
    project.target.size = 20;
    std::sync::Arc::make_mut(&mut project.imported)
        .insert("project.json".into(), std::sync::Arc::new(imported));
    for index in 0..4096 {
        project
            .notes
            .insert(format!("0x{index:x}"), "n".repeat(512));
    }
    let messages: Vec<_> = (0..40)
        .map(|_| Message {
            id: None,
            role: "assistant".into(),
            text: "response ".repeat(1024),
            activity: None,
        })
        .collect();
    let output = directory.path().join("benchmark.prism");
    let started = Instant::now();
    match mode.as_str() {
        "baseline" => baseline_save(&output, &project, &messages)?,
        "candidate" => save(&output, &project, &messages)?,
        _ => return Err("PRISM_MEMORY_IMPL must be baseline or candidate".into()),
    }
    let elapsed = started.elapsed();
    let archive = super::super::archive::Archive::open(&output)?;
    let raw_bytes: u64 = archive.manifest.entries.iter().map(|entry| entry.raw).sum();
    println!(
        "project_save mode={mode} raw_bytes={raw_bytes} archive_bytes={} elapsed_ms={:.3}",
        std::fs::metadata(&output)?.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    black_box((project, messages));
    Ok(())
}
