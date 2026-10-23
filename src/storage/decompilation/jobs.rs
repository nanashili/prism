use super::inventory::inventory_digest;
use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::{OptionalExtension, Transaction, params};

pub(crate) struct DecompilationJobSpec<'a> {
    pub id: &'a str,
    pub project: &'a str,
    pub provider: &'a str,
    pub selection: &'a str,
    pub options: &'a str,
    pub expected_inventory_digest: &'a str,
    pub filter: &'a str,
    pub addresses: Option<&'a [String]>,
}

#[derive(Clone, Debug)]
pub(crate) struct DecompilationJob {
    pub id: String,
    pub project: String,
    pub provider: String,
    pub selection: String,
    pub options: String,
    pub state: String,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub next_offset: usize,
    pub artifact_bytes: u64,
    pub error: Option<String>,
}

impl Store {
    pub(crate) fn create_decompilation_job_with_items(
        &self,
        spec: DecompilationJobSpec<'_>,
    ) -> Result<bool> {
        let transaction = self.db.unchecked_transaction()?;
        let changed = transaction.execute(
            "INSERT OR IGNORE INTO decompilation_jobs
             (id,project,provider,selection,options,state,total,created,updated)
             VALUES(?1,?2,?3,?4,?5,'paused',0,unixepoch(),unixepoch())",
            params![
                spec.id,
                spec.project,
                spec.provider,
                spec.selection,
                spec.options
            ],
        )?;
        if changed == 0 {
            transaction.commit()?;
            return Ok(false);
        }
        let actual_inventory_digest =
            inventory_digest(&transaction, spec.project, spec.filter, spec.addresses)?;
        if actual_inventory_digest != spec.expected_inventory_digest {
            return Err(failure(
                "The project function inventory changed before the job snapshot was created",
            ));
        }
        snapshot_items(&transaction, &spec)?;
        let total: i64 = transaction.query_row(
            "SELECT count(*) FROM decompilation_items WHERE job=?1",
            [spec.id],
            |row| row.get(0),
        )?;
        transaction.execute(
            "UPDATE decompilation_jobs SET total=?2,updated=unixepoch() WHERE id=?1",
            params![spec.id, total],
        )?;
        transaction.commit()?;
        Ok(true)
    }

    pub(crate) fn decompilation_job(&self, id: &str) -> Result<Option<DecompilationJob>> {
        self.db
            .query_row(
                "SELECT id,project,provider,selection,options,state,total,completed,
                        failed,next_offset,artifact_bytes,error
                 FROM decompilation_jobs WHERE id=?1",
                [id],
                |row| {
                    Ok(DecompilationJob {
                        id: row.get(0)?,
                        project: row.get(1)?,
                        provider: row.get(2)?,
                        selection: row.get(3)?,
                        options: row.get(4)?,
                        state: row.get(5)?,
                        total: usize::try_from(row.get::<_, i64>(6)?).unwrap_or(0),
                        completed: usize::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
                        failed: usize::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
                        next_offset: usize::try_from(row.get::<_, i64>(9)?).unwrap_or(0),
                        artifact_bytes: u64::try_from(row.get::<_, i64>(10)?).unwrap_or(0),
                        error: row.get(11)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn decompilation_item(
        &self,
        id: &str,
        position: usize,
    ) -> Result<Option<(String, String)>> {
        self.db
            .query_row(
                "SELECT address,module FROM decompilation_items
                 WHERE job=?1 AND position=?2",
                params![id, i64::try_from(position)?],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn update_decompilation_state(
        &self,
        id: &str,
        state: &str,
        error: Option<&str>,
    ) -> Result<()> {
        self.db.execute(
            "UPDATE decompilation_jobs SET state=?2,error=?3,updated=unixepoch() WHERE id=?1",
            params![id, state, error],
        )?;
        Ok(())
    }

    pub(crate) fn list_decompilation_jobs(
        &self,
        project: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<DecompilationJob>> {
        let mut query = self.db.prepare(
            "SELECT id,project,provider,selection,options,state,total,completed,
                    failed,next_offset,artifact_bytes,error
             FROM decompilation_jobs WHERE project=?1
             ORDER BY updated DESC,id LIMIT ?2 OFFSET ?3",
        )?;
        let rows = query.query_map(
            params![
                project,
                i64::try_from(limit.min(100))?,
                i64::try_from(offset)?
            ],
            job_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub(crate) fn retry_decompilation_failures(&self, id: &str) -> Result<usize> {
        let transaction = self.db.unchecked_transaction()?;
        let first: Option<i64> = transaction.query_row(
            "SELECT min(i.position) FROM decompilation_results r
                 JOIN decompilation_items i ON i.job=r.job AND i.address=r.address
                 WHERE r.job=?1 AND r.status='failed'",
            [id],
            |row| row.get::<_, Option<i64>>(0),
        )?;
        let Some(first) = first else {
            transaction.commit()?;
            return Ok(0);
        };
        transaction.execute(
            "UPDATE decompilation_jobs SET next_offset=?2,state='paused',
             error=NULL,updated=unixepoch() WHERE id=?1",
            params![id, first],
        )?;
        transaction.commit()?;
        usize::try_from(first).map_err(Into::into)
    }
}

fn snapshot_items(transaction: &Transaction<'_>, spec: &DecompilationJobSpec<'_>) -> Result<()> {
    if let Some(addresses) = spec.addresses {
        let mut insert = transaction.prepare(
            "INSERT INTO decompilation_items(job,position,address,module)
             SELECT ?1,?2,f.address,
                    coalesce(nullif(s.owner,''),nullif(s.path,''),'Unmapped')
             FROM functions f LEFT JOIN function_sources s
               ON s.project=f.project AND s.address=f.address
             WHERE f.project=?3 AND f.address=?4",
        )?;
        for (position, address) in addresses.iter().enumerate() {
            if insert.execute(params![
                spec.id,
                i64::try_from(position)?,
                spec.project,
                address
            ])? != 1
            {
                return Err(failure("Every selected address must belong to the project"));
            }
        }
        return Ok(());
    }

    transaction.execute(
        "INSERT INTO decompilation_items(job,position,address,module)
         SELECT ?1,row_number() OVER (ORDER BY f.address)-1,f.address,
                coalesce(nullif(s.owner,''),nullif(s.path,''),'Unmapped')
         FROM functions f LEFT JOIN function_sources s
           ON s.project=f.project AND s.address=f.address
         WHERE f.project=?2 AND (?3='' OR instr(lower(f.name),lower(?3))>0
           OR instr(f.address,?3)>0)",
        params![spec.id, spec.project, spec.filter],
    )?;
    Ok(())
}

fn job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DecompilationJob> {
    Ok(DecompilationJob {
        id: row.get(0)?,
        project: row.get(1)?,
        provider: row.get(2)?,
        selection: row.get(3)?,
        options: row.get(4)?,
        state: row.get(5)?,
        total: usize::try_from(row.get::<_, i64>(6)?).unwrap_or(0),
        completed: usize::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
        failed: usize::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
        next_offset: usize::try_from(row.get::<_, i64>(9)?).unwrap_or(0),
        artifact_bytes: u64::try_from(row.get::<_, i64>(10)?).unwrap_or(0),
        error: row.get(11)?,
    })
}
