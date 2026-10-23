use super::*;
use std::time::Instant;

fn fixture(count: usize) -> (tempfile::TempDir, Vec<Value>) {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("support/projects");
    std::fs::create_dir_all(&folder).unwrap();
    let projects: Vec<_> = (0..count)
        .map(|index| {
            json!({
                "id":format!("{index:064x}"), "name":format!("项目 {index}"),
                "original":format!("/targets/{index}/binary"), "arch":"x86_64",
                "kind":"ELF", "openedAt":index as i64
            })
        })
        .collect();
    std::fs::write(
        folder.join("index.json"),
        serde_json::to_vec(&projects).unwrap(),
    )
    .unwrap();
    (root, projects)
}

fn assert_equal(actual: &[OriginalProject], expected: &[Value]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.id, expected["id"].as_str().unwrap());
        assert_eq!(actual.name, expected["name"].as_str().unwrap());
        assert_eq!(actual.original.to_str(), expected["original"].as_str());
        assert_eq!(actual.arch, expected["arch"].as_str().unwrap());
        assert_eq!(actual.kind, expected["kind"].as_str().unwrap());
        assert_eq!(actual.opened_at, expected["openedAt"].as_i64().unwrap());
    }
}

#[test]
fn index_preserves_metadata_across_buffer_boundaries() {
    let (root, expected) = fixture(100);
    assert_equal(&index(root.path()).unwrap(), &expected);
    let path = root.path().join("support/projects/index.json");
    std::fs::write(&path, b"[] trailing data").unwrap();
    assert!(index(root.path()).is_err());
    std::fs::write(&path, b"[{\"id\":").unwrap();
    assert!(index(root.path()).is_err());
}

#[test]
#[ignore = "release benchmark: cargo test --release --locked --bin prism benchmark_project_index -- --ignored --nocapture --test-threads=1"]
fn benchmark_project_index() {
    if cfg!(debug_assertions) {
        panic!("run benchmarks with --release");
    }
    let (root, expected) = fixture(2000);
    let path = root.path().join("support/projects/index.json");
    let bytes = std::fs::metadata(&path).unwrap().len();
    let mut before = Vec::new();
    let mut after = Vec::new();
    for round in 0..8 {
        for buffered in if round % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let started = Instant::now();
            let actual: Vec<OriginalProject> = if buffered {
                index(root.path()).unwrap()
            } else {
                serde_json::from_reader(File::open(&path).unwrap()).unwrap()
            };
            let elapsed = started.elapsed();
            assert_equal(&actual, &expected);
            if round > 0 {
                if buffered { &mut after } else { &mut before }.push(elapsed);
            }
        }
    }
    before.sort();
    after.sort();
    eprintln!(
        "project_index projects=2000 bytes={bytes} samples=7 baseline_median_us={:.3} current_median_us={:.3} speedup={:.2} baseline_range_us={:.3}..{:.3} current_range_us={:.3}..{:.3}",
        before[3].as_secs_f64() * 1e6,
        after[3].as_secs_f64() * 1e6,
        before[3].as_secs_f64() / after[3].as_secs_f64(),
        before[0].as_secs_f64() * 1e6,
        before[6].as_secs_f64() * 1e6,
        after[0].as_secs_f64() * 1e6,
        after[6].as_secs_f64() * 1e6,
    );
}
