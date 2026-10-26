use crate::error::{Result, finish};
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

#[cfg_attr(feature = "profiling", hotpath::measure)]
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
        let cleanup = temporary.close().map_err(Into::into);
        return finish(Err(original), cleanup, "removing the temporary file failed");
    }

    publish(temporary, path)?;

    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn publish(mut temporary: tempfile::NamedTempFile, destination: &Path) -> Result<()> {
    // From this point the file contains complete, synced recovery data. Even a
    // failure to clear temporary-file attributes must not remove it on drop.
    temporary.disable_cleanup(true);
    let Err(failure) = temporary.persist(destination) else {
        return Ok(());
    };

    let recovery = failure.file.path().to_owned();
    let retention = failure.file.keep().err().map(|failure| failure.error);
    Err(Box::new(PublishFailure {
        cause: failure.error,
        recovery,
        retention,
    }))
}

#[derive(Debug)]
struct PublishFailure {
    cause: std::io::Error,
    recovery: PathBuf,
    retention: Option<std::io::Error>,
}

impl std::fmt::Display for PublishFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}; the completed file is preserved at {}",
            self.cause,
            self.recovery.display()
        )?;
        if let Some(error) = &self.retention {
            write!(
                formatter,
                "; clearing temporary-file attributes failed: {error}"
            )?;
        }
        Ok(())
    }
}

impl Error for PublishFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

#[cfg(test)]
mod tests {
    use super::write;
    use crate::error::{Result, failure};
    use std::io::Write;

    #[test]
    fn failed_preparation_preserves_the_destination_and_removes_partial_output() -> Result<()> {
        let folder = tempfile::tempdir()?;
        let destination = folder.path().join("document");
        std::fs::write(&destination, b"original")?;

        let error = write(&destination, |file| {
            file.write_all(b"partial")?;
            Err(failure("simulated serialization failure"))
        })
        .expect_err("failed preparation must not publish partial output");

        assert!(
            error.to_string().contains("serialization failure"),
            "{error}"
        );
        assert_eq!(std::fs::read(&destination)?, b"original");
        assert_eq!(
            std::fs::read_dir(folder.path())?.count(),
            1,
            "remove incomplete temporary output"
        );
        Ok(())
    }

    #[test]
    fn failed_publish_retains_completed_output_and_the_original_cause() -> Result<()> {
        let folder = tempfile::tempdir()?;
        let destination = folder.path().join("directory");
        std::fs::create_dir(&destination)?;

        let error = write(&destination, |file| {
            file.write_all(b"completed")?;
            Ok(())
        })
        .expect_err("a file cannot replace a directory");

        let recovery = std::fs::read_dir(folder.path())?
            .collect::<std::io::Result<Vec<_>>>()?
            .into_iter()
            .find(|entry| entry.path().is_file())
            .ok_or("the completed recovery file must remain available")?;
        assert_eq!(std::fs::read(recovery.path())?, b"completed");
        assert!(
            destination.is_dir(),
            "failed publication must preserve the destination"
        );
        assert!(
            error
                .to_string()
                .contains(&recovery.path().display().to_string()),
            "{error}"
        );
        assert!(
            error.source().is_some(),
            "retain the filesystem error for diagnostics: {error}"
        );
        Ok(())
    }
}
