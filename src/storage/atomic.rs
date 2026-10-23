use crate::error::{Result, failure};
use std::fs::File;
use std::path::Path;

pub(crate) fn write(path: &Path, operation: impl FnOnce(&mut File) -> Result<()>) -> Result<()> {
    let parent = path
        .parent()
        .ok_or("The destination has no parent folder")?;
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;

    let prepared = operation(temporary.as_file_mut()).and_then(|()| {
        temporary.as_file().sync_all()?;
        Ok(())
    });
    if let Err(original) = prepared {
        return match temporary.close() {
            Ok(()) => Err(original),
            Err(cleanup) => Err(failure(format!(
                "{original}; removing the temporary file failed: {cleanup}"
            ))),
        };
    }
    if let Err(original) = temporary.persist(path) {
        let cause = original.error;
        return match original.file.keep() {
            Ok((_, recovery)) => Err(failure(format!(
                "{cause}; the completed file is preserved at {}",
                recovery.display()
            ))),
            Err(recovery) => Err(failure(format!(
                "{cause}; retaining the temporary file also failed: {recovery}"
            ))),
        };
    }

    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}
