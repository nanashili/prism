use crate::error::{Result, failure};
use rusqlite::{Connection, OpenFlags};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub(crate) struct ImportLock {
    _file: File,
}

pub(crate) fn lock(destination: &Path) -> Result<ImportLock> {
    std::fs::create_dir_all(destination)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(destination.join("import.lock"))?;
    file.try_lock()
        .map_err(|error| failure(format!("Prism's data is already in use: {error}")))?;
    Ok(ImportLock { _file: file })
}

pub(super) fn ensure_closed(destination: &Path) -> Result<()> {
    let path = destination.join("window.json");
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    #[derive(serde::Deserialize)]
    struct Endpoint {
        address: std::net::SocketAddr,
    }
    let endpoint: Endpoint = serde_json::from_slice(&bytes)?;
    if !endpoint.address.ip().is_loopback() {
        return Err(failure(
            "The Rust window endpoint must use a loopback address",
        ));
    }
    match std::net::TcpStream::connect_timeout(
        &endpoint.address,
        std::time::Duration::from_millis(250),
    ) {
        Ok(_) => Err(failure("Close the Prism window before importing data")),
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn create(destination: &Path) -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or("Your home folder is unavailable")?;
    let source = std::env::var_os("PRISM_IMPORT_SOURCE")
        .ok_or("Set PRISM_IMPORT_SOURCE to the separate source profile to import")?;
    let source = import_source(Path::new(&source), destination)?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let root = destination.join("imports").join(format!("prism-{stamp}"));
    std::fs::create_dir_all(root.parent().ok_or("Missing import folder")?)?;
    std::fs::create_dir(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
    }
    copy_support(&source, &root.join("support"))?;
    copy_documents(&source, &root)?;
    let cached = home.join("Library/Caches/prism/analysis");
    if cached.is_dir() {
        copy_tree(&cached, &root.join("cache-analysis"))?;
    }
    std::fs::write(root.join("snapshot-complete"), b"1\n")?;
    println!("Original data preserved in {}", root.display());
    Ok(root)
}

fn import_source(source: &Path, destination: &Path) -> Result<PathBuf> {
    let source = source.canonicalize()?;
    let destination = destination.canonicalize()?;
    if !source.is_dir() {
        return Err(failure("The import source must be a profile directory"));
    }
    if source.starts_with(&destination) || destination.starts_with(&source) {
        return Err(failure(
            "Import source and destination folders must not overlap",
        ));
    }
    Ok(source)
}

fn copy_support(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir(destination)?;
    for item in std::fs::read_dir(source)? {
        let item = item?;
        let name = item.file_name();
        let text = name.to_string_lossy();
        if matches!(text.as_ref(), "engines" | "device-id")
            || text.starts_with("reconstruction.sqlite")
        {
            continue;
        }
        copy_tree(&item.path(), &destination.join(name))?;
    }
    let reconstruction = source.join("reconstruction.sqlite");
    if reconstruction.is_file() {
        backup(&reconstruction, &destination.join("reconstruction.sqlite"))?;
    }
    Ok(())
}

fn copy_documents(source: &Path, root: &Path) -> Result<()> {
    let registry = source.join("documents.json");
    if !registry.is_file() {
        return Ok(());
    }
    let documents: std::collections::BTreeMap<String, PathBuf> =
        serde_json::from_reader(std::io::BufReader::new(File::open(registry)?))?;
    std::fs::create_dir(root.join("documents"))?;
    for (id, path) in documents {
        super::projects::validate_id(&id)?;
        if !path.is_file() {
            return Err(failure(format!(
                "A saved document is missing: {}",
                path.display()
            )));
        }
        copy_file(&path, &root.join("documents").join(format!("{id}.prism")))?;
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(source)?;
    if metadata.is_dir() {
        std::fs::create_dir(destination)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        copy_file(source, destination)?;
    } else if metadata.file_type().is_symlink() {
        return Err(failure(format!(
            "Import needs review of linked data: {}",
            source.display()
        )));
    }
    Ok(())
}

pub(super) fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let result = std::process::Command::new("/bin/cp")
            .arg("-c")
            .arg(source)
            .arg(destination)
            .output()?;
        if !result.status.success() {
            return Err(failure(format!(
                "Could not copy {}: {}",
                source.display(),
                String::from_utf8_lossy(&result.stderr)
            )));
        }
    }
    #[cfg(not(target_os = "macos"))]
    std::fs::copy(source, destination)?;
    File::open(destination)?.sync_all()?;
    Ok(())
}

pub(super) fn backup_native(destination: &Path, root: &Path) -> Result<()> {
    let recovery = root.join("rust-before-import");
    if recovery.join("complete").is_file() {
        return Ok(());
    }
    if recovery.exists() {
        return Err(failure(
            "A previous recovery backup is incomplete; preserve it and use a fresh snapshot",
        ));
    }
    std::fs::create_dir(&recovery)?;
    for name in ["prism.db", "prism.views.db"] {
        let path = destination.join(name);
        if path.is_file() {
            backup(&path, &recovery.join(name))?;
        }
    }
    let settings = destination.join("settings.json");
    if settings.is_file() {
        copy_file(&settings, &recovery.join("settings.json"))?;
    }
    std::fs::write(recovery.join("complete"), b"1\n")?;
    Ok(())
}

fn backup(source: &Path, destination: &Path) -> Result<()> {
    let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.backup("main", destination, None)?;
    let copy = Connection::open_with_flags(destination, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let integrity: String = copy.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(failure(format!(
            "Database backup failed verification: {integrity}"
        )));
    }
    Ok(())
}
