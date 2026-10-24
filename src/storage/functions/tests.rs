use super::*;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[test]
fn exact_lookup_preserves_project_scope_and_ambiguity() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(&directory.path().join("prism.db"))?;
    store.db.execute_batch(
        "INSERT INTO functions VALUES
         ('project','0x10','0x10','function'),
         ('project','0x20','duplicate','function'),
         ('project','0x30','duplicate','function'),
         ('project','0x40','0x20','function'),
         ('project','0x50','0x40','function'),
         ('project','0x60','duplicate','function'),
         ('other','0x50','duplicate','function');",
    )?;

    assert_eq!(store.find_function("project", "0x10")?, ["0x10"]);
    assert_eq!(store.find_function("project", "0x20")?, ["0x20", "0x40"]);
    assert_eq!(store.find_function("project", "0x40")?, ["0x40", "0x50"]);
    assert_eq!(
        store.find_function("project", "duplicate")?,
        ["0x20", "0x30"]
    );
    assert_eq!(store.find_function("other", "duplicate")?, ["0x50"]);
    assert!(store.find_function("project", "Duplicate")?.is_empty());
    assert!(store.find_function("project", "absent")?.is_empty());
    assert!(store.find_function("absent", "duplicate")?.is_empty());

    store.db.execute(
        "UPDATE functions SET name='0x60' WHERE project='project' AND address='0x10'",
        [],
    )?;
    // Consumers resolve an unordered set: exact matches may sort after names.
    assert_eq!(store.find_function("project", "0x60")?, ["0x60", "0x10"]);
    store.db.execute(
        "INSERT INTO functions VALUES('project',NULL,'invalid','function')",
        [],
    )?;
    assert!(store.find_function("project", "invalid").is_err());
    Ok(())
}

#[test]
fn opening_existing_store_adds_name_index_without_changing_functions() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("prism.db");
    {
        let db = rusqlite::Connection::open(&path)?;
        db.execute_batch(
            "CREATE TABLE functions(project TEXT,address TEXT,name TEXT,kind TEXT,
             PRIMARY KEY(project,address));
             INSERT INTO functions VALUES('project','0x10','entry','function');",
        )?;
    }
    for _ in 0..2 {
        let store = Store::open(&path)?;
        assert_eq!(store.find_function("project", "entry")?, ["0x10"]);
        let columns: Vec<String> = store
            .db
            .prepare("PRAGMA index_info(function_name)")?
            .query_map([], |row| row.get(2))?
            .collect::<rusqlite::Result<_>>()?;
        assert_eq!(columns, ["project", "name", "address"]);
    }
    Ok(())
}

fn baseline_lookup(store: &Store, reference: &str) -> Result<Vec<String>> {
    let mut query = store.db.prepare_cached(
        "SELECT address FROM functions WHERE project=?1 AND (address=?2 OR name=?2) LIMIT 2",
    )?;
    Ok(query
        .query_map(params!["benchmark", reference], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

fn populate(store: &mut Store, rows: usize) -> Result<Duration> {
    let started = Instant::now();
    let mut import = store.begin_functions("benchmark")?;
    for start in (0..rows).step_by(512) {
        let functions = (start..(start + 512).min(rows))
            .map(|index| Procedure {
                address: format!("0x{index:08x}"),
                name: if index % 2 == 0 {
                    "shared_name".into()
                } else {
                    format!("function_{index}")
                },
                kind: "function".into(),
            })
            .collect::<Vec<_>>();
        import.append(&functions)?;
    }
    assert_eq!(import.finish()?, rows);
    Ok(started.elapsed())
}

fn median(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

#[test]
#[ignore = "release-mode performance benchmark; run with --ignored --nocapture --test-threads=1"]
fn benchmark_exact_function_lookup() -> Result<()> {
    const ROWS: usize = 200_000;
    const SAMPLES: usize = 21;
    const FAST_ITERATIONS: u32 = 1_000;
    let directory = tempfile::tempdir()?;
    let mut baseline = Store::open(&directory.path().join("baseline.db"))?;
    baseline.db.execute("DROP INDEX function_name", [])?;
    let mut candidate = Store::open(&directory.path().join("candidate.db"))?;
    let baseline_import = populate(&mut baseline, ROWS)?;
    let candidate_import = populate(&mut candidate, ROWS)?;
    for (name, store, elapsed) in [
        ("baseline", &baseline, baseline_import),
        ("candidate", &candidate, candidate_import),
    ] {
        let size: i64 = store.db.query_row(
            "SELECT (SELECT page_count FROM pragma_page_count)*
                    (SELECT page_size FROM pragma_page_size)",
            [],
            |row| row.get(0),
        )?;
        println!(
            "function_lookup fixture={name} rows={ROWS} import_ms={:.3} database_bytes={size}",
            elapsed.as_secs_f64() * 1000.0
        );
    }
    for (name, reference) in [
        ("address", "0x00030d3f"),
        ("name", "function_199999"),
        ("missing", "missing_function"),
        ("100k_duplicate_names", "shared_name"),
    ] {
        assert_eq!(
            baseline_lookup(&baseline, reference)?,
            candidate.find_function("benchmark", reference)?
        );
        // Warm SQLite pages and prepared statements before collecting samples.
        black_box(baseline_lookup(&baseline, reference)?);
        black_box(candidate.find_function("benchmark", reference)?);
        let mut before = Vec::with_capacity(SAMPLES);
        let mut after = Vec::with_capacity(SAMPLES);
        for sample in 0..SAMPLES {
            // Alternate order to reduce systematic effects of drift.
            let mut measure = |old: bool| -> Result<()> {
                let started = Instant::now();
                if old {
                    black_box(baseline_lookup(&baseline, black_box(reference))?);
                    before.push(started.elapsed());
                } else {
                    for _ in 0..FAST_ITERATIONS {
                        black_box(candidate.find_function("benchmark", black_box(reference))?);
                    }
                    after.push(started.elapsed() / FAST_ITERATIONS);
                }
                Ok(())
            };
            measure(sample % 2 == 0)?;
            measure(sample % 2 != 0)?;
        }
        let before = median(&mut before).as_secs_f64() * 1000.0;
        let after = median(&mut after).as_secs_f64() * 1000.0;
        println!(
            "function_lookup case={name} samples={SAMPLES} baseline_ms={before:.6} candidate_ms={after:.6} speedup={:.2}x",
            before / after
        );
    }
    Ok(())
}
