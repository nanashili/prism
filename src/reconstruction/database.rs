use crate::error::{Result, failure};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub(crate) struct Database {
    pub root: PathBuf,
    pub db: Connection,
}

impl Database {
    pub fn open(root: &Path, create: DatabaseOpen) -> Result<Self> {
        let create = create == DatabaseOpen::Create;

        if !root.is_absolute() || !root.is_dir() {
            return Err(failure("Choose an existing absolute reconstruction folder"));
        }
        let root = root.canonicalize()?;
        let metadata = root.join(".prism-reconstruction");
        if create {
            std::fs::create_dir_all(&metadata)?;
        }
        if std::fs::symlink_metadata(&metadata)?
            .file_type()
            .is_symlink()
        {
            return Err(failure("Reconstruction metadata cannot be a symbolic link"));
        }
        let path = metadata.join("state.sqlite");
        if !create && !path.is_file() {
            return Err(failure(
                "This folder has no Prism reconstruction generations",
            ));
        }
        if path.exists() && std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(failure(
                "The reconstruction database cannot be a symbolic link",
            ));
        }
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(3))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL; PRAGMA cache_size=-2048;
             PRAGMA max_page_count=524288;",
        )?;
        super::schema::initialize(&db)?;
        Ok(Self { root, db })
    }

    pub fn generation(&self, requested: Option<i64>) -> Result<i64> {
        let generation = match requested {
            Some(id) => id,
            None => {
                self.db
                    .query_row("SELECT coalesce(max(id),0) FROM generations", [], |row| {
                        row.get(0)
                    })?
            }
        };
        if generation <= 0 {
            return Err(failure("Start a reconstruction generation first"));
        }
        self.db.query_row(
            "SELECT id FROM generations WHERE id=?1",
            [generation],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(generation)
    }

    pub fn verify_project(&self, project: &str) -> Result<()> {
        let matches: bool = self.db.query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM generations WHERE project<>?1)",
            [project],
            |row| row.get(0),
        )?;
        if !matches {
            return Err(failure(
                "This reconstruction belongs to a different input binary",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DatabaseOpen {
    Create,
    Existing,
}

impl From<bool> for DatabaseOpen {
    fn from(value: bool) -> Self {
        if value { Self::Create } else { Self::Existing }
    }
}
