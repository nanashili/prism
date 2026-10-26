use super::Settings;

#[test]
fn changing_native_rea_path_invalidates_the_analysis_backend() {
    let original = Settings {
        provider: "ghidra-rust-rea".into(),
        rea: "/opt/rea-rust-a".into(),
        ..Settings::default()
    };

    let mut changed = original.clone();
    changed.rea = "/opt/rea-rust-b".into();

    assert!(!original.same_analysis_backend(&changed));
}
