use crate::error::{Result, failure};
use std::path::{Path, PathBuf};

pub(crate) fn create(parent: &Path, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name
            .chars()
            .any(|character| !character.is_alphanumeric() && !matches!(character, '-' | '_'))
    {
        return Err(failure(
            "Use letters, numbers, underscores or hyphens in the project name",
        ));
    }
    if !parent.is_absolute() || !parent.is_dir() {
        return Err(failure("Choose an existing absolute output folder"));
    }
    let root = parent.join(name);
    std::fs::create_dir(&root)?;
    Ok(root)
}
