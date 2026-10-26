use super::set_colour_tag;
use crate::model::Project;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

#[test]
fn colour_edits_share_unmodified_documents_and_preserve_undo_states() {
    let document = Arc::new(json!({"instructions": "mov rax, rbx\n".repeat(4096)}));
    let mut project = Project::default();
    Arc::make_mut(&mut project.imported).insert("large.json".into(), document.clone());
    let mut history = Vec::new();
    for edit in 0..20 {
        history.push(project.clone());
        set_colour_tag(&mut project, "0x10", &format!("colour-{edit}"));
    }
    for (edit, previous) in history.iter().enumerate() {
        assert!(Arc::ptr_eq(&previous.imported["large.json"], &document));
        if edit == 0 {
            assert!(!previous.imported.contains_key("prism-color-tags"));
        } else {
            assert_eq!(
                previous.imported["prism-color-tags"]["0x10"],
                format!("colour-{}", edit - 1)
            );
        }
    }
    assert!(Arc::ptr_eq(&project.imported["large.json"], &document));
    assert_eq!(project.imported["prism-color-tags"]["0x10"], "colour-19");
    let saved = serde_json::to_value(&project).expect(
        "colour edits share unmodified documents and preserve undo states: valid fixture operation",
    );
    let restored: Project = serde_json::from_value(saved.clone()).expect(
        "colour edits share unmodified documents and preserve undo states: valid fixture operation",
    );
    assert_eq!(serde_json::to_value(restored).expect("colour edits share unmodified documents and preserve undo states: valid fixture operation"), saved);
    assert_eq!(saved["imported"]["large.json"], *document);
    set_colour_tag(&mut project, "0x10", "");
    assert!(project.imported["prism-color-tags"]["0x10"].is_null());
    assert_eq!(
        history[19].imported["prism-color-tags"]["0x10"],
        "colour-18"
    );
}

#[test]
fn invalid_tag_metadata_is_preserved() {
    let mut project = Project::default();
    Arc::make_mut(&mut project.imported)
        .insert("prism-color-tags".into(), Arc::new(json!(["reference"])));
    set_colour_tag(&mut project, "0x10", "red");
    assert_eq!(*project.imported["prism-color-tags"], json!(["reference"]));
}

#[test]
#[ignore = "fresh-process memory workload; PRISM_MEMORY_IMPL=baseline|candidate and PRISM_MEMORY_FIXTURE=<JSON>"]
fn memory_project_undo() {
    let implementation =
        std::env::var("PRISM_MEMORY_IMPL").expect("memory project undo: valid fixture operation");
    let fixture = std::env::var("PRISM_MEMORY_FIXTURE")
        .expect("memory project undo: valid fixture operation");
    let source =
        std::fs::File::open(fixture).expect("memory project undo: valid fixture operation");
    let document: Value = serde_json::from_reader(std::io::BufReader::new(source))
        .expect("memory project undo: valid fixture operation");
    let checksum = json_checksum(&document);
    let started = std::time::Instant::now();
    if implementation == "baseline" {
        // cc2c4f6's imported map COW cloned every Value on each tag edit.
        let mut imported: Arc<BTreeMap<String, Value>> =
            Arc::new(BTreeMap::from([("large.json".into(), document)]));
        let mut history = Vec::new();
        for edit in 0..20 {
            history.push(imported.clone());
            let tags = Arc::make_mut(&mut imported)
                .entry("prism-color-tags".into())
                .or_insert_with(|| json!({}));
            tags.as_object_mut()
                .expect("memory project undo: valid fixture operation")
                .insert("0x10".into(), format!("colour-{edit}").into());
        }
        let elapsed = started.elapsed();
        assert_eq!(imported["prism-color-tags"]["0x10"], "colour-19");
        for (edit, previous) in history.iter().enumerate() {
            assert_eq!(json_checksum(&previous["large.json"]), checksum);
            if edit == 0 {
                assert!(!previous.contains_key("prism-color-tags"));
            } else {
                assert_eq!(
                    previous["prism-color-tags"]["0x10"],
                    format!("colour-{}", edit - 1)
                );
            }
        }
        assert_eq!(json_checksum(&imported["large.json"]), checksum);
        println!(
            "memory_project_undo impl=baseline edits=20 retained_document_versions=21 checksum={checksum} elapsed_ms={:.3}",
            elapsed.as_secs_f64() * 1000.0
        );
        std::hint::black_box((&imported, &history));
    } else {
        assert_eq!(implementation, "candidate");
        let mut project = Project::default();
        Arc::make_mut(&mut project.imported).insert("large.json".into(), Arc::new(document));
        let mut history = Vec::new();
        for edit in 0..20 {
            history.push(project.clone());
            set_colour_tag(&mut project, "0x10", &format!("colour-{edit}"));
        }
        let elapsed = started.elapsed();
        assert!(
            history.iter().all(|old| Arc::ptr_eq(
                &old.imported["large.json"],
                &project.imported["large.json"]
            ))
        );
        assert_eq!(project.imported["prism-color-tags"]["0x10"], "colour-19");
        for (edit, previous) in history.iter().enumerate() {
            assert_eq!(json_checksum(&previous.imported["large.json"]), checksum);
            if edit == 0 {
                assert!(!previous.imported.contains_key("prism-color-tags"));
            } else {
                assert_eq!(
                    previous.imported["prism-color-tags"]["0x10"],
                    format!("colour-{}", edit - 1)
                );
            }
        }
        assert_eq!(json_checksum(&project.imported["large.json"]), checksum);
        println!(
            "memory_project_undo impl=candidate edits=20 retained_document_versions=1 checksum={checksum} elapsed_ms={:.3}",
            elapsed.as_secs_f64() * 1000.0
        );
        std::hint::black_box((&project, &history));
    }
}

fn json_checksum(value: &Value) -> String {
    use sha2::{Digest, Sha256};

    struct HashWriter(Sha256);
    impl std::io::Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = HashWriter(Sha256::new());
    serde_json::to_writer(&mut writer, value).expect("flush: valid fixture operation");
    format!("{:x}", writer.0.finalize())
}
