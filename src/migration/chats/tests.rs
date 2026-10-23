use super::*;
use serde_json::json;

#[test]
fn project_lookup_is_lazy_and_preserves_explicit_ids() {
    let root = tempfile::tempdir().unwrap();
    let mut projects = ProjectLookup::default();
    assert_eq!(
        projects
            .resolve(root.path(), Some("explicit".into()), Some("/target"))
            .unwrap(),
        Some("explicit".into())
    );
    assert!(
        projects
            .resolve(root.path(), None, Some("/target"))
            .is_err()
    );
}

#[test]
fn project_lookup_reads_once_and_keeps_first_exact_path_match() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("support/projects");
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("index.json");
    let records: Vec<_> = ["first", "duplicate"].into_iter().map(|id| json!({
        "id":id, "name":"项目", "original":"/target//binary", "arch":"x86_64", "kind":"ELF", "openedAt":0
    })).collect();
    std::fs::write(&path, serde_json::to_vec(&records).unwrap()).unwrap();
    let mut projects = ProjectLookup::default();
    assert_eq!(
        projects
            .resolve(root.path(), None, Some("/target//binary"))
            .unwrap(),
        Some("first".into())
    );
    // Removing the fixture makes an accidental second index read fail.
    std::fs::remove_file(path).unwrap();
    assert_eq!(
        projects
            .resolve(root.path(), None, Some("/target//binary"))
            .unwrap(),
        Some("first".into())
    );
    assert_eq!(
        projects
            .resolve(root.path(), None, Some("/target/binary"))
            .unwrap(),
        None
    );
    assert_eq!(projects.resolve(root.path(), None, None).unwrap(), None);
    assert_eq!(
        projects
            .resolve(
                root.path(),
                Some("explicit".into()),
                Some("/target//binary")
            )
            .unwrap(),
        Some("explicit".into())
    );
}

fn thread(id: &str) -> serde_json::Value {
    json!({
        "id":id,"title":format!("Chat {id}"),"project":"project","updatedAt":1000,
        "messages":[{"kind":"user","text":"hello 🦀"}]
    })
}

fn import_test_bytes(bytes: &[u8], store: &Store, root: &Path, report: &mut Report) -> Result<()> {
    use std::io::Write;
    let mut temporary = tempfile::tempfile()?;
    temporary.write_all(bytes)?;
    import_archived_threads(
        &mut temporary,
        root,
        store,
        report,
        &mut ProjectLookup::default(),
    )
}

#[test]
fn streamed_chat_import_validates_all_types_and_trailing_data_before_writes() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("prism.db");
    let store = Store::open(&path)?;
    store.prepare_upstream_import()?;
    let query = rusqlite::Connection::open(&path)?;
    let mut invalid = thread("second");
    invalid["updatedAt"] = "not a timestamp".into();
    let invalid_type = serde_json::to_vec(&vec![thread("first"), invalid])?;
    let mut trailing = serde_json::to_vec(&vec![thread("first"), thread("second")])?;
    trailing.extend_from_slice(b" trailing");
    for bytes in [invalid_type, trailing] {
        let mut report = Report::default();
        assert!(import_test_bytes(&bytes, &store, root.path(), &mut report).is_err());
        assert_eq!(report.chats_added, 0);
        assert_eq!(
            query.query_row("SELECT count(*) FROM chats", [], |row| row.get::<_, i64>(0))?,
            0
        );
    }
    Ok(())
}

#[test]
fn streamed_chat_import_preserves_order_retry_and_per_thread_failures() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("prism.db");
    let store = Store::open(&path)?;
    store.prepare_upstream_import()?;
    let bytes = serde_json::to_vec(&vec![thread("first"), thread("second")])?;
    let mut report = Report::default();
    import_test_bytes(&bytes, &store, root.path(), &mut report)?;
    assert_eq!((report.chats_added, report.messages_added), (2, 2));
    let mut retry = Report::default();
    import_test_bytes(&bytes, &store, root.path(), &mut retry)?;
    assert_eq!(
        (
            retry.chats_added,
            retry.messages_added,
            retry.skipped_existing
        ),
        (0, 0, 2)
    );
    let query = rusqlite::Connection::open(&path)?;
    let titles = query
        .prepare("SELECT title FROM chats ORDER BY id")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    assert_eq!(titles, ["Chat first", "Chat second"]);

    let mut missing_project = thread("fourth");
    missing_project.as_object_mut().unwrap().remove("project");
    let bytes = serde_json::to_vec(&vec![thread("third"), missing_project])?;
    let mut report = Report::default();
    assert!(import_test_bytes(&bytes, &store, root.path(), &mut report).is_err());
    assert_eq!((report.chats_added, report.messages_added), (1, 1));
    assert_eq!(
        query.query_row("SELECT count(*) FROM chats", [], |row| row.get::<_, i64>(0))?,
        3
    );
    Ok(())
}

#[test]
#[ignore = "fresh-process RSS workload; requires PRISM_MEMORY_FIXTURE array of archived threads"]
fn memory_archive_chat_import() -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::hint::black_box;
    use std::time::Instant;

    let input = std::env::var_os("PRISM_MEMORY_FIXTURE").ok_or("Set PRISM_MEMORY_FIXTURE")?;
    let mode = std::env::var("PRISM_MEMORY_IMPL").unwrap_or_else(|_| "candidate".into());
    let root = tempfile::tempdir()?;
    let path = root.path().join("prism.db");
    let store = Store::open(&path)?;
    store.prepare_upstream_import()?;
    let mut temporary = tempfile::tempfile()?;
    let raw_bytes = std::io::copy(&mut File::open(input)?, &mut temporary)?;
    temporary.rewind()?;
    let mut projects = ProjectLookup::default();
    let mut report = Report::default();
    let started = Instant::now();
    match mode.as_str() {
        "baseline" => {
            let threads: Vec<Thread> = serde_json::from_reader(BufReader::new(temporary))?;
            for thread in threads {
                insert(root.path(), &store, thread, &mut report, &mut projects)?;
            }
        }
        "candidate" => import_archived_threads(
            &mut temporary,
            root.path(),
            &store,
            &mut report,
            &mut projects,
        )?,
        _ => return Err("PRISM_MEMORY_IMPL must be baseline or candidate".into()),
    }
    let elapsed = started.elapsed();
    let query = rusqlite::Connection::open(&path)?;
    let mut statement = query.prepare(
        "SELECT c.id,c.title,c.project,m.role,m.text FROM chats c JOIN messages m ON m.chat=c.id ORDER BY m.id"
    )?;
    let mut rows = statement.query([])?;
    let mut digest = Sha256::new();
    let mut row_count = 0;
    while let Some(row) = rows.next()? {
        digest.update(row.get::<_, i64>(0)?.to_le_bytes());
        for column in 1..5 {
            let value: String = row.get(column)?;
            digest.update((value.len() as u64).to_le_bytes());
            digest.update(value.as_bytes());
        }
        row_count += 1;
    }
    assert_eq!(row_count, report.messages_added);
    println!(
        "archive_chat_import mode={mode} raw_bytes={raw_bytes} chats={} messages={} digest={:x} elapsed_ms={:.3}",
        report.chats_added,
        report.messages_added,
        digest.finalize(),
        elapsed.as_secs_f64() * 1000.0
    );
    black_box(report);
    Ok(())
}
