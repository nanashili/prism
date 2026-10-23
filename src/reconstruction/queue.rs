use super::database::Database;
use crate::error::{Result, failure};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};

impl Database {
    pub fn claim(&mut self, generation: i64, worker: &str) -> Result<Value> {
        if worker.is_empty() || worker.len() > 120 {
            return Err(failure("Supply a worker name of 1–120 bytes"));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        active(&tx, generation)?;
        expire(&tx, generation)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT token FROM batches WHERE generation=?1 AND worker=?2 AND state='leased'",
                params![generation, worker],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(token) = existing {
            let result = batch(&tx, &token)?;
            tx.commit()?;
            return Ok(result);
        }
        let available: bool = tx.query_row(
            "SELECT (SELECT count(*) FROM batches WHERE generation=?1 AND state='leased')<workers
             FROM generations WHERE id=?1",
            [generation],
            |row| row.get(0),
        )?;
        if !available {
            tx.commit()?;
            return Ok(json!({"available":false,"reason":"Worker limit reached"}));
        }
        let module = next_module(&tx, generation)?;
        let Some(module) = module else {
            tx.commit()?;
            return Ok(
                json!({"available":false,"reason":"No eligible pending work. Check active or blocked batches."}),
            );
        };
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|error| failure(error.to_string()))?;
        let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        tx.execute(
            "INSERT INTO batches(token,generation,worker,state,expires) VALUES(?1,?2,?3,'leased',unixepoch()+900)",
            params![token,generation,worker],
        )?;
        tx.execute(
            "UPDATE units SET state='leased',lease=?1 WHERE generation=?2 AND address IN
             (SELECT address FROM units WHERE generation=?2 AND state='pending' AND module=?3
              ORDER BY address LIMIT (SELECT batch_size FROM generations WHERE id=?2))",
            params![token, generation, module],
        )?;
        tx.execute(
            "UPDATE modules SET state='leased' WHERE generation=?1 AND module=?2",
            params![generation, module],
        )?;
        let result = batch(&tx, &token)?;
        tx.commit()?;
        Ok(result)
    }
}

pub(super) fn active(db: &Connection, generation: i64) -> Result<()> {
    let state: String = db.query_row(
        "SELECT state FROM generations WHERE id=?1",
        [generation],
        |row| row.get(0),
    )?;
    if state != "active" {
        return Err(failure(format!(
            "Generation is {state}. Resume it before claiming work"
        )));
    }
    Ok(())
}

pub(super) fn expire(db: &Connection, generation: i64) -> Result<()> {
    db.execute(
        "UPDATE units SET state='pending',lease=NULL WHERE generation=?1 AND state='leased'
         AND lease IN (SELECT token FROM batches WHERE generation=?1 AND expires<=unixepoch())",
        [generation],
    )?;
    db.execute("UPDATE batches SET state='expired' WHERE generation=?1 AND state='leased' AND expires<=unixepoch()", [generation])?;
    db.execute(
        "UPDATE modules SET state='pending' WHERE generation=?1 AND state='leased'
         AND NOT EXISTS(SELECT 1 FROM units WHERE generation=?1 AND module=modules.module AND state='leased')",
        [generation],
    )?;
    Ok(())
}

fn next_module(db: &Connection, generation: i64) -> Result<Option<String>> {
    let scaffold: String = db.query_row(
        "SELECT state FROM units WHERE generation=?1 AND address='__project__'",
        [generation],
        |row| row.get(0),
    )?;
    if !matches!(scaffold.as_str(), "generated" | "reused") {
        return Ok((scaffold == "pending").then(String::new));
    }
    Ok(db
        .query_row(
            "SELECT m.module FROM modules m
         WHERE m.generation=?1 AND m.state='pending'
           AND m.contract_ready=1
           AND EXISTS(SELECT 1 FROM units u WHERE u.generation=m.generation
                     AND u.module=m.module AND u.state='pending')
           AND (m.cycle_group='' OR NOT EXISTS(
             SELECT 1 FROM modules member WHERE member.generation=m.generation
               AND member.cycle_group=m.cycle_group AND member.contract_ready=0))
           AND NOT EXISTS(
             SELECT 1 FROM module_dependencies d JOIN modules dependency
               ON dependency.generation=d.generation AND dependency.module=d.depends_on
             WHERE d.generation=m.generation AND d.module=m.module
               AND (dependency.state <> 'complete' OR dependency.contract_ready=0)
               AND (m.cycle_group='' OR dependency.cycle_group<>m.cycle_group))
         ORDER BY m.module LIMIT 1",
            [generation],
            |row| row.get(0),
        )
        .optional()?)
}

pub(super) fn batch(db: &Connection, token: &str) -> Result<Value> {
    let mut query = db.prepare(
        "SELECT address,name,module,kind,evidence FROM units WHERE lease=?1 ORDER BY address",
    )?;
    let rows = query.query_map([token], |row| {
        Ok(json!({
            "address":row.get::<_, String>(0)?,"name":row.get::<_, String>(1)?,
            "module":row.get::<_, String>(2)?,"kind":row.get::<_, String>(3)?,
            "evidence_ready":row.get::<_, Option<String>>(4)?.is_some(),
        }))
    })?;
    let units = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(json!({"available":true,"lease":token,"lease_seconds":900,"units":units}))
}
