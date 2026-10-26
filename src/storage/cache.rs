use super::Store;
use crate::error::{Result, failure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::path::Path;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(super) fn open(path: &Path) -> Result<Connection> {
    let cache = Connection::open(path)?;
    cache.busy_timeout(std::time::Duration::from_secs(3))?;
    cache.execute_batch(
        "PRAGMA auto_vacuum=FULL;
        PRAGMA cache_size=-1024;
        CREATE TABLE IF NOT EXISTS views(
            project TEXT, address TEXT, kind TEXT, data BLOB, accessed INTEGER,
            PRIMARY KEY(project,address,kind));
        CREATE INDEX IF NOT EXISTS views_accessed ON views(accessed);
        CREATE TABLE IF NOT EXISTS cache_usage(
            id INTEGER PRIMARY KEY CHECK(id=1),
            bytes INTEGER NOT NULL CHECK(bytes>=0));
        CREATE TRIGGER IF NOT EXISTS views_usage_insert AFTER INSERT ON views BEGIN
            UPDATE cache_usage SET bytes=bytes+coalesce(length(new.data),0) WHERE id=1;
        END;
        CREATE TRIGGER IF NOT EXISTS views_usage_delete AFTER DELETE ON views BEGIN
            UPDATE cache_usage SET bytes=bytes-coalesce(length(old.data),0) WHERE id=1;
        END;
        CREATE TRIGGER IF NOT EXISTS views_usage_update AFTER UPDATE OF data ON views BEGIN
            UPDATE cache_usage SET bytes=bytes-coalesce(length(old.data),0)
                +coalesce(length(new.data),0) WHERE id=1;
        END;",
    )?;
    // Existing installations need one inventory scan; triggers maintain this from then on.
    cache.execute(
        "INSERT INTO cache_usage(id,bytes)
         SELECT 1,(SELECT coalesce(sum(length(data)),0) FROM views)
         WHERE NOT EXISTS(SELECT 1 FROM cache_usage WHERE id=1)",
        [],
    )?;
    Ok(cache)
}

impl Store {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn cache(&self, project: &str, address: &str, kind: &str) -> Result<Option<String>> {
        let cached: Option<(Vec<u8>, bool)> = self
            .cache
            .query_row(
                "SELECT data,accessed IS NULL OR accessed < unixepoch()-60
                 FROM views WHERE project=?1 AND address=?2 AND kind=?3",
                params![project, address, kind],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((bytes, touch)) = cached else {
            return Ok(None);
        };
        if touch {
            self.cache.execute(
                "UPDATE views SET accessed=unixepoch()
                 WHERE project=?1 AND address=?2 AND kind=?3
                 AND (accessed IS NULL OR accessed < unixepoch()-60)",
                params![project, address, kind],
            )?;
        }
        let raw = crate::project::archive::decode(&bytes, 2 * 1024 * 1024)?;
        Ok(Some(String::from_utf8(raw)?))
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn cache_view(
        &self,
        project: &str,
        address: &str,
        kind: &str,
        text: &str,
        budget: u64,
    ) -> Result<()> {
        if text.len() > 2 * 1024 * 1024 {
            return Err(failure("The view exceeds the 2 MiB cache entry limit"));
        }
        if budget == 0 {
            return self.trim_cache(0);
        }
        let compressed = zstd::encode_all(text.as_bytes(), 3)?;
        let transaction = self.cache.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO views VALUES(?1,?2,?3,?4,unixepoch())
             ON CONFLICT(project,address,kind) DO UPDATE
             SET data=excluded.data,accessed=excluded.accessed",
            params![project, address, kind, compressed],
        )?;
        trim(&transaction, budget)?;
        transaction.commit()?;
        Ok(())
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn trim_cache(&self, budget: u64) -> Result<()> {
        let transaction = self.cache.unchecked_transaction()?;
        trim(&transaction, budget)?;
        transaction.commit()?;
        Ok(())
    }
}

fn trim(transaction: &Transaction<'_>, budget: u64) -> Result<()> {
    let total: i64 =
        transaction.query_row("SELECT bytes FROM cache_usage WHERE id=1", [], |row| {
            row.get(0)
        })?;
    let mut remaining = u64::try_from(total)?;
    while remaining > budget {
        let removed: i64 = transaction.query_row(
            "DELETE FROM views
             WHERE rowid=(SELECT rowid FROM views ORDER BY accessed,rowid LIMIT 1)
             RETURNING coalesce(length(data),0)",
            [],
            |row| row.get(0),
        )?;
        remaining = remaining
            .checked_sub(u64::try_from(removed)?)
            .ok_or("Invalid cache payload accounting")?;
    }
    Ok(())
}
