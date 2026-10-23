use super::library;
use crate::error::{Result, failure};
use crate::settings::skill_library::SkillLibrary;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub(in crate::ui) const GAME_LIBRARY: &str = "https://github.com/1llu50ry/mcp-gamehacking";

pub(super) struct Repository {
    url: String,
    owner: String,
    name: String,
}

impl Repository {
    pub fn parse(text: &str) -> Result<Self> {
        let url = url::Url::parse(text.trim())?;
        if url.scheme() != "https"
            || url.host_str() != Some("github.com")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(
                "Use a public repository URL such as https://github.com/owner/repository".into(),
            );
        }
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        let [owner, name] = parts.as_slice() else {
            return Err("Use the repository URL, without a branch or file path".into());
        };
        let name = name.strip_suffix(".git").unwrap_or(name);
        if !valid_component(owner) || !valid_component(name) {
            return Err("The repository owner or name is invalid".into());
        }
        Ok(Self {
            url: format!("https://github.com/{owner}/{name}").to_ascii_lowercase(),
            owner: owner.to_ascii_lowercase(),
            name: name.to_ascii_lowercase(),
        })
    }

    fn destination(&self, storage: &Path) -> PathBuf {
        storage.join(&self.owner).join(&self.name)
    }
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && !value.starts_with('-')
        && value.len() <= 100
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

pub(in crate::ui) fn install(
    url: &str,
    storage: &Path,
    cancelled: &AtomicBool,
) -> Result<SkillLibrary> {
    let repository = Repository::parse(url)?;
    let destination = repository.destination(storage);
    if destination.exists() {
        return restore_download(&destination, &repository.url);
    }
    let parent = destination
        .parent()
        .ok_or("The library has no parent folder")?;
    std::fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".download-")
        .tempdir_in(parent)?;
    let prepared = prepare_download(&repository, staging.path(), &destination, cancelled);
    let library = match prepared {
        Ok(library) => library,
        Err(original) => return cleanup_failure(staging, original),
    };
    if let Err(error) = check_cancelled(cancelled) {
        return cleanup_failure(staging, error);
    }
    if let Err(error) = std::fs::rename(staging.path().join("repository"), &destination) {
        return cleanup_failure(staging, error.into());
    }
    if let Err(error) = staging.close() {
        return Err(failure(format!(
            "The library was downloaded to {} but temporary folder cleanup failed: {error}. Retry to connect the completed download.",
            destination.display()
        )));
    }
    Ok(library)
}

fn prepare_download(
    repository: &Repository,
    staging: &Path,
    destination: &Path,
    cancelled: &AtomicBool,
) -> Result<SkillLibrary> {
    let checkout = fetch_checkout(repository, staging, cancelled)?;
    let revision = checkout_revision(&checkout, cancelled)?;
    validate_checkout(&checkout, staging)?;
    check_cancelled(cancelled)?;
    std::fs::remove_dir_all(checkout.join(".git"))?;
    let library = SkillLibrary {
        path: destination.to_path_buf(),
        repository: Some(repository.url.clone()),
        revision: Some(revision),
        ..Default::default()
    };
    crate::storage::atomic::write(&checkout.join(".prism-library.json"), |file| {
        serde_json::to_writer_pretty(file, &library)?;
        Ok(())
    })?;
    Ok(library)
}

fn fetch_checkout(
    repository: &Repository,
    staging: &Path,
    cancelled: &AtomicBool,
) -> Result<PathBuf> {
    git(
        staging,
        &[
            "clone",
            "--depth=1",
            "--filter=blob:none",
            "--no-checkout",
            "--no-tags",
            "--",
            &repository.url,
            "repository",
        ],
        cancelled,
    )?;
    let checkout = staging.join("repository");
    git(
        &checkout,
        &[
            "sparse-checkout",
            "set",
            "--no-cone",
            "--",
            "**/SKILL.md",
            "**/LICENSE*",
            "**/NOTICE*",
        ],
        cancelled,
    )?;
    git(&checkout, &["checkout", "--force"], cancelled)?;
    Ok(checkout)
}

fn checkout_revision(checkout: &Path, cancelled: &AtomicBool) -> Result<String> {
    let revision = git(checkout, &["rev-parse", "HEAD"], cancelled)?
        .trim()
        .to_owned();
    if revision.len() != 40 || !revision.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Git did not return a valid repository revision".into());
    }
    Ok(revision)
}

fn validate_checkout(checkout: &Path, staging: &Path) -> Result<()> {
    let candidate = SkillLibrary {
        path: checkout.to_path_buf(),
        ..Default::default()
    };
    let index = library::index(&staging.join("no-custom-skills"), &[candidate]);
    if index.entries.is_empty() {
        return Err(failure(format!(
            "The repository could not be imported as a skill library. {}",
            index.warnings.join("; ")
        )));
    }
    Ok(())
}

fn restore_download(destination: &Path, url: &str) -> Result<SkillLibrary> {
    let manifest = std::fs::read(destination.join(".prism-library.json"))?;
    let library: SkillLibrary = serde_json::from_slice(&manifest)?;
    if library.repository.as_deref() != Some(url) || library.path != destination {
        return Err("The download folder belongs to another library".into());
    }
    Ok(library)
}

pub(in crate::ui) fn remove(library: &SkillLibrary, storage: &Path) -> Result<()> {
    let repository = Repository::parse(
        library
            .repository
            .as_deref()
            .ok_or("This is a local library")?,
    )?;
    let expected = repository.destination(storage);
    if library.path != expected
        || std::fs::symlink_metadata(&expected)?.is_symlink()
        || expected.canonicalize()?
            != storage
                .canonicalize()?
                .join(&repository.owner)
                .join(&repository.name)
    {
        return Err("The download path changed. Remove this library manually.".into());
    }
    let manifest = restore_download(&expected, &repository.url)?;
    remove_library_files(&expected)?;
    finish_removal(&expected, &manifest)
}

fn remove_library_files(folder: &Path) -> Result<()> {
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let result = entry.map_err(Into::into).and_then(|entry| -> Result<()> {
            if entry.file_name() == ".prism-library.json" {
                return Ok(());
            }
            if entry.file_type()?.is_dir() {
                std::fs::remove_dir_all(entry.path())?;
            } else {
                std::fs::remove_file(entry.path())?;
            }
            Ok(())
        });
        if let Err(error) = result {
            failures.push(error.to_string());
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failure(format!(
            "Some library files could not be removed. The library record was kept so you can retry: {}",
            failures.join("; ")
        )))
    }
}

fn finish_removal(folder: &Path, library: &SkillLibrary) -> Result<()> {
    let manifest = folder.join(".prism-library.json");
    std::fs::remove_file(&manifest)?;
    if let Err(original) = std::fs::remove_dir(folder) {
        let recovery = crate::storage::atomic::write(&manifest, |file| {
            serde_json::to_writer_pretty(file, library)?;
            Ok(())
        });
        return match recovery {
            Ok(()) => Err(original.into()),
            Err(recovery) => Err(failure(format!(
                "{original}; restoring the library record also failed: {recovery}"
            ))),
        };
    }
    Ok(())
}

pub(in crate::ui) fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        Err("Skill library operation cancelled".into())
    } else {
        Ok(())
    }
}

fn git(folder: &Path, arguments: &[&str], cancelled: &AtomicBool) -> Result<String> {
    check_cancelled(cancelled)?;
    let empty_config = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut command = Command::new(crate::executable_paths::find("git"));
    command
        .current_dir(folder)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", empty_config)
        .args([
            "-c",
            &format!("core.hooksPath={empty_config}"),
            "-c",
            "protocol.allow=never",
            "-c",
            "protocol.https.allow=always",
        ])
        .args(arguments);
    crate::protocol::process::capture(&mut command, cancelled, Duration::from_secs(180))
}

fn cleanup_failure<T>(
    staging: tempfile::TempDir,
    original: Box<dyn std::error::Error + Send + Sync>,
) -> Result<T> {
    match staging.close() {
        Ok(()) => Err(original),
        Err(cleanup) => Err(failure(format!(
            "{original}; removing the incomplete download also failed: {cleanup}"
        ))),
    }
}
