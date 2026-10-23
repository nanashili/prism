use crate::error::{Result, failure};
use crate::settings::support_dir;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

struct Entry {
    path: PathBuf,
    bytes: u64,
    accessed: SystemTime,
    protected: bool,
}

pub(crate) fn touch(root: &Path) -> Result<()> {
    crate::storage::atomic::write(&root.join(".accessed"), |_| Ok(()))
}

pub(crate) fn reserve(
    incoming: u64,
    budget: u64,
    protected: &[&Path],
    cancelled: &AtomicBool,
) -> Result<()> {
    if incoming > budget {
        return Err(failure(
            "This output exceeds the generated-file cache budget in Settings",
        ));
    }
    let protected = retained_paths(protected)?;
    let mut entries = inventory(&protected, cancelled)?;
    let mut total = incoming;
    for entry in &entries {
        total = total
            .checked_add(entry.bytes)
            .ok_or("Cache size overflowed")?;
    }
    entries.sort_by_key(|entry| entry.accessed);
    let mut failures = Vec::new();
    for entry in entries {
        if total <= budget {
            break;
        }
        check_cancelled(cancelled)?;
        if entry.protected {
            continue;
        }
        match std::fs::remove_dir_all(&entry.path) {
            Ok(()) => total -= entry.bytes,
            Err(error) => failures.push(format!("{}: {error}", entry.path.display())),
        }
    }
    if total > budget {
        failures.push("The open input and new output exceed the generated-file cache budget. Raise it in Settings or open a smaller file first.".into());
    }
    if !failures.is_empty() {
        return Err(failure(failures.join("; ")));
    }
    Ok(())
}

fn retained_paths(paths: &[&Path]) -> Result<Vec<PathBuf>> {
    let mut retained = Vec::new();
    for path in paths {
        match path.canonicalize() {
            Ok(path) => retained.push(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // An input removed outside the app has no cache entry to retain.
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(retained)
}

fn inventory(protected: &[PathBuf], cancelled: &AtomicBool) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut visited = 0;
    for category in ["documents", "android"] {
        let root = support_dir().join(category);
        match std::fs::symlink_metadata(&root) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(failure("Generated cache folders cannot be symbolic links"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
        let directory = match std::fs::read_dir(&root) {
            Ok(directory) => directory,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        for item in directory {
            check_cancelled(cancelled)?;
            count_entry(&mut visited)?;
            let item = item?;
            let name = item.file_name();
            let name = name.to_string_lossy();
            if name.len() != 64 || !name.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                continue;
            }
            if !item.file_type()?.is_dir() {
                return Err(failure("A generated cache entry is not a directory"));
            }
            let path = item.path().canonicalize()?;
            let accessed = match std::fs::metadata(path.join(".accessed")) {
                Ok(metadata) => metadata.modified()?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    item.metadata()?.modified()?
                }
                Err(error) => return Err(error.into()),
            };
            entries.push(Entry {
                bytes: directory_bytes(&path, &mut visited, cancelled)?,
                protected: protected.iter().any(|input| input.starts_with(&path)),
                path,
                accessed,
            });
        }
    }
    Ok(entries)
}

pub(crate) fn directory_bytes(
    root: &Path,
    visited: &mut usize,
    cancelled: &AtomicBool,
) -> Result<u64> {
    let mut pending = vec![root.to_path_buf()];
    let mut total = 0u64;
    while let Some(directory) = pending.pop() {
        for item in std::fs::read_dir(directory)? {
            check_cancelled(cancelled)?;
            count_entry(visited)?;
            let item = item?;
            let kind = item.file_type()?;
            if kind.is_dir() {
                pending.push(item.path());
            } else {
                total = total
                    .checked_add(std::fs::symlink_metadata(item.path())?.len())
                    .ok_or("Cache size overflowed")?;
            }
        }
    }
    Ok(total)
}

fn count_entry(visited: &mut usize) -> Result<()> {
    *visited += 1;
    if *visited > 100_000 {
        return Err(failure(
            "The generated cache contains more than 100,000 entries",
        ));
    }
    Ok(())
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(failure("Cancelled"));
    }
    Ok(())
}
