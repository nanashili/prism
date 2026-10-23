use crate::error::{Result, failure};
use std::fs::{File, OpenOptions};

/// The OS releases this lock on process exit, including crashes. Each window
/// opens a separate handle, so a project has at most one durable batch runner.
pub(super) struct Ownership {
    _file: File,
}

impl Ownership {
    pub(super) fn acquire(project: &str) -> Result<Self> {
        if project.len() != 64 || !project.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(failure("A valid project identity is required"));
        }
        let directory = crate::settings::support_dir().join("decompilation-locks");
        std::fs::create_dir_all(&directory)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join(format!("{project}.lock")))?;
        file.try_lock().map_err(|error| {
            failure(format!(
                "Another window is processing this project's durable job: {error}"
            ))
        })?;
        Ok(Self { _file: file })
    }
}
