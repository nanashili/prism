use super::database::Database;
use crate::error::{Result, failure};
use rusqlite::params;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct Snapshot {
    pub id: i64,
    pub revision: i64,
    pub path: PathBuf,
    pub language: String,
}

impl Database {
    pub(super) fn snapshot(&mut self, generation: i64, cancelled: &AtomicBool) -> Result<Snapshot> {
        let tx = self.db.transaction()?;
        let (revision, language): (i64, String) = tx.query_row(
            "SELECT revision,language FROM generations WHERE id=?1",
            [generation],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM files WHERE generation=?1",
            [generation],
            |row| row.get(0),
        )?;
        if count == 0 {
            return Err(failure(
                "This generation has no source files to export or build",
            ));
        }
        let parent = self.root.join("outputs");
        std::fs::create_dir_all(&parent)?;
        if std::fs::symlink_metadata(&parent)?.file_type().is_symlink() {
            return Err(failure(
                "The output snapshot folder cannot be a symbolic link",
            ));
        }
        let path = tempfile::Builder::new()
            .prefix(&format!("generation-{generation}-revision-{revision}-"))
            .tempdir_in(&parent)?
            .keep();
        let id_result = tx.execute(
            "INSERT INTO builds(generation,revision,snapshot) VALUES(?1,?2,?3)",
            params![generation, revision, path.to_string_lossy()],
        );
        if let Err(error) = id_result {
            return Err(failure(format!(
                "{error}. Empty snapshot preserved at {}",
                path.display()
            )));
        }
        let id = tx.last_insert_rowid();
        let result = export_files(&tx, generation, &path, cancelled);
        commit_export(tx, id, &path, result)?;
        Ok(Snapshot {
            id,
            revision,
            path,
            language,
        })
    }
}

fn commit_export(
    transaction: rusqlite::Transaction<'_>,
    id: i64,
    path: &std::path::Path,
    result: Result<()>,
) -> Result<()> {
    let mut problems = Vec::new();
    if let Err(error) = result {
        problems.push(error.to_string());
        if let Err(error) = transaction.execute(
            "UPDATE builds SET succeeded=0,output=?1 WHERE id=?2",
            params![problems[0], id],
        ) {
            problems.push(format!("Could not record export failure: {error}"));
        }
    }
    if let Err(error) = transaction.commit() {
        problems.push(format!("Could not commit snapshot receipt: {error}"));
    }
    if !problems.is_empty() {
        return Err(failure(format!(
            "{}. Snapshot preserved at {}",
            problems.join("; "),
            path.display()
        )));
    }
    Ok(())
}

fn export_files(
    db: &rusqlite::Connection,
    generation: i64,
    destination: &std::path::Path,
    cancelled: &AtomicBool,
) -> Result<()> {
    let mut query =
        db.prepare("SELECT path,digest FROM files WHERE generation=?1 ORDER BY path")?;
    let mut rows = query.query([generation])?;
    while let Some(row) = rows.next()? {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Cancelled"));
        }
        let path: String = row.get(0)?;
        let digest: String = row.get(1)?;
        super::objects::validate_path(&path)?;
        if destination.join(&path).exists() {
            return Err(failure(format!(
                "{path} collides with an exported path on this filesystem"
            )));
        }
        let bytes = super::objects::get(db, &digest)?;
        crate::storage::atomic::write(&destination.join(path), |file| {
            file.write_all(&bytes)?;
            Ok(())
        })?;
    }
    Ok(())
}
