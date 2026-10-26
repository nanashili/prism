use crate::error::{Result, failure};
use std::fs::{File, OpenOptions};
use std::path::Path;

/// Holds the process-wide lock for Prism's current support profile.
pub(crate) struct ProfileLock {
    _file: File,
}

pub(crate) fn lock_profile(directory: &Path) -> Result<ProfileLock> {
    std::fs::create_dir_all(directory)?;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("profile.lock"))?;
    file.try_lock()
        .map_err(|error| failure(format!("Prism's profile is already in use: {error}")))?;

    Ok(ProfileLock { _file: file })
}
