use super::Store;
use crate::error::{Result, failure};
use rusqlite::{OptionalExtension, params};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub(crate) struct ResourceEntry {
    pub path: String,
    pub name: String,
    pub folder: bool,
}

#[derive(Default)]
pub(crate) struct ResourcePage {
    pub entries: Vec<ResourceEntry>,
    pub total: usize,
    pub more: bool,
    pub available: bool,
}

impl Store {
    pub(crate) fn resource_page(
        &mut self,
        root: Option<&Path>,
        parent: &str,
        offset: usize,
        cancelled: &AtomicBool,
    ) -> Result<ResourcePage> {
        let Some(root) = root else {
            return Ok(ResourcePage::default());
        };
        if parent.is_empty() && offset == 0 {
            self.db
                .execute_batch("DROP TABLE IF EXISTS temp.resource_identity")?;
        }
        self.prepare_resources(root, cancelled)?;
        let mut statement = self.db.prepare(
            "SELECT path,name,folder FROM sidebar_resources WHERE parent=?1
             ORDER BY folder DESC,name COLLATE NOCASE LIMIT 301 OFFSET ?2",
        )?;
        let rows = statement.query_map(params![parent, i64::try_from(offset)?], |row| {
            Ok(ResourceEntry {
                path: row.get(0)?,
                name: row.get(1)?,
                folder: row.get(2)?,
            })
        })?;
        let mut entries: Vec<_> = rows.collect::<std::result::Result<_, _>>()?;
        let more = entries.len() > 300;
        entries.truncate(300);
        let count: u32 = self.db.query_row(
            "SELECT count(*) FROM sidebar_resources WHERE folder=0",
            [],
            |row| row.get(0),
        )?;
        Ok(ResourcePage {
            entries,
            total: count as usize,
            more,
            available: true,
        })
    }

    fn prepare_resources(&mut self, root: &Path, cancelled: &AtomicBool) -> Result<()> {
        self.db
            .execute_batch("CREATE TEMP TABLE IF NOT EXISTS resource_identity(root TEXT)")?;
        let previous: Option<String> = self
            .db
            .query_row("SELECT root FROM resource_identity LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        if previous.as_deref() == Some(root.to_string_lossy().as_ref()) {
            return Ok(());
        }
        let transaction = self.db.transaction()?;
        transaction.execute_batch(
            "DROP TABLE IF EXISTS temp.sidebar_resources;
             CREATE TEMP TABLE sidebar_resources(path TEXT PRIMARY KEY,parent TEXT,name TEXT,folder INTEGER);
             CREATE INDEX temp.resource_parent ON sidebar_resources(parent,folder,name);
             DELETE FROM resource_identity;",
        )?;
        index_resources(&transaction, root, cancelled)?;
        transaction.execute(
            "INSERT INTO resource_identity VALUES(?1)",
            [root.to_string_lossy()],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn index_resources(
    db: &rusqlite::Transaction<'_>,
    root: &Path,
    cancelled: &AtomicBool,
) -> Result<()> {
    let mut stack = vec![(PathBuf::new(), std::fs::read_dir(root)?)];
    let mut insert = db.prepare("INSERT INTO sidebar_resources VALUES(?1,?2,?3,?4)")?;
    let mut count = 0;
    while let Some((parent, directory)) = stack.last_mut() {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Resource indexing cancelled"));
        }
        let Some(entry) = directory.next() else {
            stack.pop();
            continue;
        };
        let entry = entry?;
        let kind = entry.file_type()?;
        if !kind.is_file() && !kind.is_dir() {
            continue;
        }
        count += 1;
        if count > 400_000 {
            return Err(failure("This app exceeds the 400,000 resource entry limit"));
        }
        let relative = parent.join(entry.file_name());
        insert.execute(params![
            relative.to_string_lossy(),
            parent.to_string_lossy(),
            entry.file_name().to_string_lossy(),
            kind.is_dir()
        ])?;
        if kind.is_dir() {
            if stack.len() >= 64 {
                return Err(failure("This app exceeds the 64 folder depth limit"));
            }
            stack.push((relative, std::fs::read_dir(entry.path())?));
        }
    }
    Ok(())
}
