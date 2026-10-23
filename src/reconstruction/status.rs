use super::database::Database;
use crate::error::Result;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

impl Database {
    pub fn status(&self, generation: i64) -> Result<Value> {
        let generation_row = self.db.query_row(
            "SELECT id,parent,project,language,instructions,state,created,revision,batch_size,workers,
                    analyzer,model,prompt_version,adapter_version,validation_version
             FROM generations WHERE id=?1", [generation], |row| Ok(json!({
                "id":row.get::<_, i64>(0)?,"parent":row.get::<_, Option<i64>>(1)?,
                "project":row.get::<_, String>(2)?,"language":row.get::<_, String>(3)?,
                "instructions":row.get::<_, String>(4)?,"state":row.get::<_, String>(5)?,
                "created":row.get::<_, i64>(6)?,"revision":row.get::<_, i64>(7)?,
                "batch_size":row.get::<_, i64>(8)?,"workers":row.get::<_, i64>(9)?,
                "analyzer":row.get::<_, String>(10)?,"model":row.get::<_, String>(11)?,
                "prompt_version":row.get::<_, String>(12)?,
                "adapter_version":row.get::<_, String>(13)?,
                "validation_version":row.get::<_, String>(14)?,
            })),
        )?;
        let mut counts =
            json!({"pending":0,"leased":0,"generated":0,"reused":0,"blocked":0,"total":0});
        let mut query = self
            .db
            .prepare("SELECT state,count(*) FROM units WHERE generation=?1 GROUP BY state")?;
        let mut rows = query.query([generation])?;
        let mut total = 0i64;
        while let Some(row) = rows.next()? {
            let state: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            counts[&state] = count.into();
            total += count;
        }
        counts["total"] = total.into();
        let expired: i64 = self.db.query_row(
            "SELECT count(*) FROM units u JOIN batches b ON b.token=u.lease
             WHERE u.generation=?1 AND u.state='leased' AND b.expires<=unixepoch()",
            [generation],
            |row| row.get(0),
        )?;
        counts["leased"] = (counts["leased"].as_i64().unwrap_or_default() - expired).into();
        counts["pending"] = (counts["pending"].as_i64().unwrap_or_default() + expired).into();
        let metrics = self.db.query_row(
            "SELECT count(*),coalesce(sum(finished-created),0) FROM batches WHERE generation=?1 AND state='submitted'",
            [generation], |row| Ok(json!({"submitted_batches":row.get::<_, i64>(0)?,"batch_wall_seconds":row.get::<_, i64>(1)?})),
        )?;
        let adapter =
            super::language::describe(generation_row["language"].as_str().unwrap_or_default())?;
        let mut actions = self.db.prepare(
            "SELECT stage,state,count(*) FROM actions WHERE generation=?1 GROUP BY stage,state",
        )?;
        let action_rows = actions
            .query_map([generation], |row| {
                Ok(json!({
                    "stage": row.get::<_, String>(0)?,
                    "state": row.get::<_, String>(1)?,
                    "count": row.get::<_, i64>(2)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let modules: i64 = self.db.query_row(
            "SELECT count(*) FROM modules WHERE generation=?1",
            [generation],
            |row| row.get(0),
        )?;
        let mut module_states = self
            .db
            .prepare("SELECT state,count(*) FROM modules WHERE generation=?1 GROUP BY state")?;
        let module_state_counts = module_states
            .query_map([generation], |row| {
                Ok(json!({
                    "state": row.get::<_, String>(0)?,
                    "count": row.get::<_, i64>(1)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let files: i64 = self.db.query_row(
            "SELECT count(*) FROM files WHERE generation=?1",
            [generation],
            |row| row.get(0),
        )?;
        let build = self.db.query_row(
            "SELECT id,revision,snapshot,succeeded,substr(output,-8192) FROM builds WHERE generation=?1 ORDER BY id DESC LIMIT 1",
            [generation], |row| Ok(json!({"id":row.get::<_, i64>(0)?,"revision":row.get::<_, i64>(1)?,
                "snapshot":row.get::<_, String>(2)?,"succeeded":row.get::<_, Option<bool>>(3)?,
                "diagnostics":row.get::<_, String>(4)?})),
        ).optional()?.map(|mut build| {
            let incomplete = counts["pending"].as_i64().unwrap_or_default()
                + counts["leased"].as_i64().unwrap_or_default()
                + counts["blocked"].as_i64().unwrap_or_default();
            build["reconstruction_complete"] = (incomplete == 0).into();
            build["stale_source"] = (incomplete > 0 && files > 0).into();
            build["incomplete_units"] = incomplete.into();
            build
        });
        let mut generations = self.db.prepare(
            "SELECT id,language,state,revision FROM generations ORDER BY id DESC LIMIT 20",
        )?;
        let history = generations
            .query_map([], |row| {
                Ok(json!({
                    "id":row.get::<_, i64>(0)?,"language":row.get::<_, String>(1)?,
                    "state":row.get::<_, String>(2)?,"revision":row.get::<_, i64>(3)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(
            json!({"root":self.root,"generation":generation_row,"adapter":adapter,
            "counts":counts,"modules":modules,"module_states":module_state_counts,
            "actions":action_rows,
            "files":files,"metrics":metrics,"latest_build":build,"history":history,"behavior_verified":false,
            "scope":"Indexed functions plus project integration. Generated is an agent claim, not verified equivalence."}),
        )
    }

    pub fn units(&self, generation: i64, state: &str, offset: u64) -> Result<Value> {
        let mut query = self.db.prepare(
            "SELECT address,name,module,kind,visible_state,reason FROM
             (SELECT u.*,CASE WHEN u.state='leased' AND b.expires<=unixepoch()
              THEN 'pending' ELSE u.state END AS visible_state
              FROM units u LEFT JOIN batches b ON b.token=u.lease WHERE u.generation=?1)
             WHERE (?2='' OR visible_state=?2) ORDER BY address LIMIT 65 OFFSET ?3",
        )?;
        let mut rows = query.query_map(params![generation,state,i64::try_from(offset)?], |row| Ok(json!({
            "address":row.get::<_, String>(0)?,"name":row.get::<_, String>(1)?,"module":row.get::<_, String>(2)?,
            "kind":row.get::<_, String>(3)?,"state":row.get::<_, String>(4)?,"reason":row.get::<_, String>(5)?,
        })))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let next_offset = (rows.len() > 64).then_some(offset + 64);
        rows.truncate(64);
        Ok(json!({"units":rows,"offset":offset,"page_size":64,"next_offset":next_offset}))
    }

    pub fn read_file(&self, generation: i64, path: &str) -> Result<Value> {
        super::objects::validate_path(path)?;
        let digest: String = self.db.query_row(
            "SELECT digest FROM files WHERE generation=?1 AND path=?2",
            params![generation, path],
            |row| row.get(0),
        )?;
        let bytes = super::objects::get(&self.db, &digest)?;
        Ok(json!({"path":path,"sha256":digest,"content":String::from_utf8(bytes)?}))
    }

    pub fn files(&self, generation: i64, offset: u64) -> Result<Value> {
        let mut query = self.db.prepare(
            "SELECT f.path,f.digest,o.size FROM files f JOIN objects o ON o.digest=f.digest
             WHERE f.generation=?1 ORDER BY f.path LIMIT 64 OFFSET ?2",
        )?;
        let rows = query.query_map(params![generation,i64::try_from(offset)?], |row| Ok(json!({
            "path":row.get::<_, String>(0)?,"sha256":row.get::<_, String>(1)?,"bytes":row.get::<_, i64>(2)?,
        })))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(json!({"files":rows,"offset":offset,"page_size":64}))
    }
}
