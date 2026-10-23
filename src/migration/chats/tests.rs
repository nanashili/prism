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
