use super::{Catalog, download, library};
use crate::error::Result;
use crate::settings::{
    Settings,
    research::{Category, Research},
    skill_library::SkillLibrary,
};
use std::path::Path;
use std::time::{Duration, Instant};

fn write_skill(root: &Path, name: &str, metadata: &str, body: &str) -> Result<()> {
    let folder = root.join(name);
    std::fs::create_dir_all(&folder)?;
    std::fs::write(
        folder.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Research reference\n{metadata}\n---\n{body}"),
    )?;
    Ok(())
}

fn ready(catalog: &mut Catalog, settings: &Settings) {
    catalog.all(settings);
    let deadline = Instant::now() + Duration::from_secs(15);
    while catalog.loading() {
        assert!(Instant::now() < deadline, "Skill indexing did not finish");
        std::thread::sleep(Duration::from_millis(5));
        catalog.poll();
    }
}

#[test]
fn large_library_filters_categories_and_expands_current_instructions_without_truncation()
-> Result<()> {
    let folder = tempfile::tempdir()?;
    let root = folder.path().join("repository");
    let skills = root.join("skills");
    for index in 0..4200 {
        write_skill(
            &skills,
            &format!("ags-native-{index}"),
            "metadata:\n  topics: [binary]",
            "Archived project summary",
        )?;
    }
    write_skill(
        &skills,
        "unreal-object-model",
        "metadata:\n  topics: [game-engine]",
        "Original instructions",
    )?;
    write_skill(
        &skills,
        "uefi-image",
        "metadata:\n  topics: [firmware]",
        "Read firmware sections",
    )?;
    let library = SkillLibrary {
        path: root,
        ..Default::default()
    };
    let mut settings = Settings::default();
    settings.desktop.skills_dir = folder.path().join("custom").to_string_lossy().into_owned();
    settings.desktop.skill_libraries.push(library);
    settings.research.categories = vec![Category::GameEngines];
    let mut catalog = Catalog::default();
    ready(&mut catalog, &settings);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    assert_eq!(
        catalog
            .all(&settings)
            .iter()
            .filter(|entry| !entry.built_in)
            .count(),
        4202
    );
    let enabled = catalog.enabled(&settings);
    let imported: Vec<_> = enabled.iter().filter(|entry| !entry.built_in).collect();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].title, "Unreal object model");
    let reference = imported[0].reference();

    let complete = format!(
        "{}\nFINAL REQUIRED STEP",
        "Read the structure. ".repeat(600)
    );
    write_skill(
        &skills,
        "unreal-object-model",
        "metadata:\n  topics: [game-engine]",
        &complete,
    )?;
    assert!(
        catalog
            .resolve_prompt(&settings, &reference)?
            .ends_with("FINAL REQUIRED STEP")
    );
    assert!(
        catalog
            .resolve_prompt(&settings, &reference)?
            .contains(&complete)
    );
    settings.research.categories = vec![Category::Web];
    assert!(catalog.resolve_prompt(&settings, &reference).is_err());
    assert!(
        catalog
            .enabled(&settings)
            .iter()
            .all(|entry| entry.built_in)
    );

    settings.research.categories = vec![Category::Binaries];
    settings.desktop.skill_libraries[0].include_archives = true;
    ready(&mut catalog, &settings);
    assert_eq!(
        catalog
            .enabled(&settings)
            .iter()
            .filter(|entry| !entry.built_in)
            .count(),
        4200
    );
    settings.desktop.skill_libraries[0].enabled = false;
    ready(&mut catalog, &settings);
    assert!(
        catalog
            .enabled(&settings)
            .iter()
            .all(|entry| entry.built_in)
    );
    Ok(())
}

#[test]
fn libraries_cannot_shadow_bundled_or_other_library_skills() -> Result<()> {
    let folder = tempfile::tempdir()?;
    let mut settings = Settings::default();
    settings.desktop.skills_dir = folder.path().join("custom").to_string_lossy().into_owned();
    for name in ["one", "two"] {
        let path = folder.path().join(name);
        write_skill(&path, "first-look", "topics: [binary]", name)?;
        settings.desktop.skill_libraries.push(SkillLibrary {
            path,
            ..Default::default()
        });
    }
    let mut catalog = Catalog::default();
    ready(&mut catalog, &settings);
    let skills = catalog.enabled(&settings);
    let first_looks: Vec<_> = skills
        .iter()
        .filter(|entry| entry.title == "First look")
        .collect();
    assert_eq!(first_looks.len(), 3);
    let identifiers: std::collections::HashSet<_> =
        first_looks.iter().map(|entry| &entry.id).collect();
    assert_eq!(identifiers.len(), 3);
    let imported = first_looks
        .iter()
        .find(|entry| !entry.built_in)
        .ok_or("Missing imported skill")?;
    settings
        .desktop
        .disabled_skills
        .push(imported.id.to_string());
    assert!(
        catalog
            .resolve_prompt(&settings, &imported.reference())
            .is_err()
    );
    assert!(
        catalog
            .enabled(&settings)
            .iter()
            .any(|entry| entry.id == "first-look")
    );
    std::fs::remove_file(imported.file.as_ref().ok_or("Missing skill file")?)?;
    settings.desktop.disabled_skills.clear();
    assert!(
        catalog
            .resolve_prompt(&settings, &imported.reference())
            .is_err()
    );
    Ok(())
}

#[test]
fn oversized_skills_are_reported_without_discarding_other_entries() -> Result<()> {
    let folder = tempfile::tempdir()?;
    write_skill(
        folder.path(),
        "readable",
        "topics: [web]",
        "Read the browser script",
    )?;
    write_skill(
        folder.path(),
        "too-large",
        "topics: [web]",
        &"x".repeat(128 * 1024),
    )?;
    let result = library::index(folder.path(), &[]);
    assert_eq!(result.entries.len(), 1);
    assert!(
        result
            .warnings
            .iter()
            .any(|message| message.contains("128 KB"))
    );
    Ok(())
}

#[test]
fn github_urls_are_validated_before_any_download_files_are_created() -> Result<()> {
    let folder = tempfile::tempdir()?;
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    for url in [
        "file:///tmp/repo",
        "http://github.com/owner/repo",
        "https://example.com/owner/repo",
        "https://user@github.com/owner/repo",
        "https://github.com/owner/repo/tree/main",
        "https://github.com/owner/%2e%2e",
        "https://github.com/owner/repo?x=1",
    ] {
        assert!(
            download::install(url, folder.path(), &cancelled).is_err(),
            "{url}"
        );
    }
    assert_eq!(std::fs::read_dir(folder.path())?.count(), 0);
    assert!(download::Repository::parse("https://github.com/1llu50ry/mcp-gamehacking.git").is_ok());
    Ok(())
}

#[test]
fn cancelled_download_cleans_staging_and_never_publishes_a_library() -> Result<()> {
    let folder = tempfile::tempdir()?;
    let cancelled = std::sync::atomic::AtomicBool::new(true);
    let error = download::install(download::GAME_LIBRARY, folder.path(), &cancelled)
        .expect_err("A cancelled download must fail");
    assert!(error.to_string().contains("cancelled"));
    assert!(!folder.path().join("1llu50ry/mcp-gamehacking").exists());
    assert_eq!(
        std::fs::read_dir(folder.path().join("1llu50ry"))?.count(),
        0
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn failed_removal_keeps_ownership_record_and_attempts_independent_files() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let folder = tempfile::tempdir()?;
    let path = folder.path().join("owner/repo");
    let protected = path.join("protected");
    std::fs::create_dir_all(&protected)?;
    std::fs::write(protected.join("skill.md"), "Keep on failure")?;
    std::fs::write(path.join("removable.md"), "Remove independently")?;
    let library = SkillLibrary {
        path: path.clone(),
        repository: Some("https://github.com/owner/repo".into()),
        ..Default::default()
    };
    std::fs::write(
        path.join(".prism-library.json"),
        serde_json::to_vec(&library)?,
    )?;
    std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o500))?;

    let removal = download::remove(&library, folder.path());
    std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o700))?;
    assert!(removal.is_err());
    assert!(path.join(".prism-library.json").exists());
    assert!(protected.join("skill.md").exists());
    assert!(!path.join("removable.md").exists());

    download::remove(&library, folder.path())?;
    assert!(!path.exists());
    Ok(())
}

#[test]
fn removing_a_download_checks_ownership_and_preserves_local_libraries() -> Result<()> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("owner/repo");
    std::fs::create_dir_all(&path)?;
    let library = SkillLibrary {
        path: path.clone(),
        repository: Some("https://github.com/owner/repo".into()),
        revision: Some("a".repeat(40)),
        ..Default::default()
    };
    std::fs::write(path.join("keep.md"), "Keep unless ownership is verified")?;
    assert!(download::remove(&library, folder.path()).is_err());
    assert!(path.join("keep.md").exists());
    std::fs::write(
        path.join(".prism-library.json"),
        serde_json::to_vec(&library)?,
    )?;
    let mut local = library.clone();
    local.repository = None;
    assert!(download::remove(&local, folder.path()).is_err());
    assert!(path.join("keep.md").exists());
    download::remove(&library, folder.path())?;
    assert!(!path.exists());
    Ok(())
}

#[test]
fn old_settings_keep_features_while_new_installations_start_setup() -> Result<()> {
    let old: Settings = serde_json::from_str("{}")?;
    assert!(old.research.setup_complete);
    assert!(old.research.contains(Category::Graphics));
    assert!(old.research.contains(Category::GameSecurity));
    let mut new = Settings::default();
    assert!(!new.research.setup_complete);
    new.research = Research {
        categories: vec![Category::Web, Category::Network],
        setup_complete: true,
    };
    let restored: Settings = serde_json::from_slice(&serde_json::to_vec(&new)?)?;
    assert_eq!(restored.research, new.research);
    let mut catalog = Catalog::default();
    let entries = catalog.enabled(&restored);
    assert!(entries.iter().any(|entry| entry.id == "network-map"));
    assert!(
        entries
            .iter()
            .all(|entry| entry.id != "shader-check" && entry.id != "decompile")
    );
    Ok(())
}

#[test]
#[ignore = "Downloads the public example library from GitHub"]
fn github_download_imports_real_library_without_source_archives() -> Result<()> {
    let folder = tempfile::tempdir()?;
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let library = download::install(download::GAME_LIBRARY, folder.path(), &cancelled)?;
    assert!(library.revision.is_some());
    assert!(!library.path.join(".git").exists());
    let index = library::index(
        &folder.path().join("custom"),
        std::slice::from_ref(&library),
    );
    assert!(index.entries.len() > 4000);
    assert!(
        index
            .warnings
            .iter()
            .all(|warning| warning.contains("128 KB")),
        "{:?}",
        index.warnings
    );
    assert!(
        index
            .entries
            .iter()
            .any(|entry| entry.title == "Unreal object model")
    );
    let mut bytes = 0;
    for entry in &index.entries {
        let file = entry.file.as_ref().ok_or("No skill file")?;
        bytes += std::fs::metadata(file)?.len();
        assert!(
            !file
                .parent()
                .ok_or("No skill folder")?
                .join("source.txt")
                .exists()
        );
    }
    println!(
        "Indexed {} skill files, {} bytes, {} oversized skills skipped; revision {}",
        index.entries.len(),
        bytes,
        index.warnings.len(),
        library.revision.as_deref().unwrap_or("unknown")
    );
    let restored = download::install(download::GAME_LIBRARY, folder.path(), &cancelled)?;
    assert_eq!(restored.path, library.path);
    download::remove(&library, folder.path())?;
    assert!(!library.path.exists());
    Ok(())
}
