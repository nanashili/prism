use super::{database::Database, submission::Submission};
use crate::error::{Result, failure};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Deserialize;
use serde_json::{Value, json};

impl Database {
    pub fn submit(&mut self, generation: i64, payload: &Value) -> Result<Value> {
        let submission = Submission::deserialize(payload)?;
        submission.validate()?;
        let receipt = super::objects::digest_json(payload)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (state, previous): (String, Option<String>) = tx.query_row(
            "SELECT state,receipt FROM batches WHERE generation=?1 AND token=?2",
            params![generation, submission.lease],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if state == "submitted" && previous.as_deref() == Some(&receipt) {
            return Ok(json!({"saved":true,"already_submitted":true}));
        }
        super::queue::active(&tx, generation)?;
        let valid: bool = tx.query_row(
            "SELECT state='leased' AND expires>unixepoch() FROM batches WHERE token=?1",
            [&submission.lease],
            |row| row.get(0),
        )?;
        if !valid {
            return Err(failure(
                "This batch expired, was released or was already submitted",
            ));
        }
        validate_units(&tx, &submission)?;
        save_files(&tx, generation, &submission)?;
        save_results(&tx, generation, &submission)?;
        refresh_modules(&tx, generation, &submission.lease)?;
        tx.execute(
            "UPDATE batches SET state='submitted',receipt=?1,finished=unixepoch() WHERE token=?2",
            params![receipt, submission.lease],
        )?;
        tx.execute(
            "UPDATE generations SET revision=revision+1 WHERE id=?1",
            [generation],
        )?;
        tx.commit()?;
        Ok(json!({"saved":true,"behavior_verified":false}))
    }
}

fn refresh_modules(db: &Connection, generation: i64, lease: &str) -> Result<()> {
    db.execute(
        "UPDATE modules SET state=CASE
            WHEN EXISTS(SELECT 1 FROM units u WHERE u.generation=modules.generation
                       AND u.module=modules.module AND u.state IN ('pending','leased')) THEN 'pending'
            WHEN EXISTS(SELECT 1 FROM units u WHERE u.generation=modules.generation
                       AND u.module=modules.module AND u.state='blocked') THEN 'blocked'
            ELSE 'complete' END
         WHERE generation=?1 AND module IN
            (SELECT DISTINCT module FROM units WHERE generation=?1 AND lease=?2)",
        params![generation, lease],
    )?;
    Ok(())
}

fn validate_units(db: &Connection, submission: &Submission) -> Result<()> {
    let count: i64 = db.query_row(
        "SELECT count(*) FROM units WHERE lease=?1",
        [&submission.lease],
        |row| row.get(0),
    )?;
    if count != i64::try_from(submission.results.len())? {
        return Err(failure(
            "Return a result for every unit in the claimed batch",
        ));
    }
    for result in &submission.results {
        let evidence: Option<Option<String>> = db
            .query_row(
                "SELECT evidence FROM units WHERE lease=?1 AND address=?2 AND state='leased'",
                params![submission.lease, result.address],
                |row| row.get(0),
            )
            .optional()?;
        let evidence = evidence.ok_or("This unit is not owned by the batch")?;
        if result.state == "generated" && result.address != "__project__" && evidence.is_none() {
            return Err(failure(
                "Read the batch evidence before claiming generated source",
            ));
        }
    }
    Ok(())
}

fn save_files(db: &Connection, generation: i64, submission: &Submission) -> Result<()> {
    for file in &submission.files {
        let previous: Option<String> = db
            .query_row(
                "SELECT digest FROM files WHERE generation=?1 AND path=?2",
                params![generation, file.path],
                |row| row.get(0),
            )
            .optional()?;
        if previous.as_deref().unwrap_or_default() != file.previous_sha256 {
            return Err(failure(format!(
                "{} changed. Read its current hash and merge before submitting",
                file.path
            )));
        }
        validate_file_location(db, generation, &file.path)?;
        let digest = super::objects::put(db, file.content.as_bytes())?;
        db.execute(
            "INSERT INTO files VALUES(?1,?2,?3) ON CONFLICT(generation,path) DO UPDATE SET digest=excluded.digest",
            params![generation,file.path,digest],
        )?;
    }
    Ok(())
}

fn validate_file_location(db: &Connection, generation: i64, path: &str) -> Result<()> {
    let prefix = format!("{path}/");
    let limit = format!("{path}0");
    let descendant: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM files WHERE generation=?1 AND path>=?2 AND path<?3)",
        params![generation, prefix, limit],
        |row| row.get(0),
    )?;
    if descendant {
        return Err(failure(format!("{path} is already used as a directory")));
    }
    for (separator, _) in path.match_indices('/') {
        let ancestor: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM files WHERE generation=?1 AND path=?2)",
            params![generation, &path[..separator]],
            |row| row.get(0),
        )?;
        if ancestor {
            return Err(failure(format!(
                "{path} has an ancestor that is already a file"
            )));
        }
    }
    Ok(())
}

fn save_results(db: &Connection, generation: i64, submission: &Submission) -> Result<()> {
    let (language, instructions, analyzer, model, prompt, adapter, validation):
        (String, String, String, String, String, String, String) = db.query_row(
        "SELECT language,instructions,analyzer,model,prompt_version,adapter_version,validation_version
         FROM generations WHERE id=?1",
        [generation],
        |row| Ok((
            row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
            row.get(4)?, row.get(5)?, row.get(6)?,
        )),
    )?;
    for result in &submission.results {
        db.execute(
            "DELETE FROM links WHERE generation=?1 AND address=?2",
            params![generation, result.address],
        )?;
        for path in &result.paths {
            db.execute(
                "INSERT OR IGNORE INTO links VALUES(?1,?2,?3)",
                params![generation, result.address, path],
            )?;
        }
        db.execute(
            "UPDATE units SET state=?1,reason=?2 WHERE generation=?3 AND address=?4",
            params![result.state, result.reason, generation, result.address],
        )?;
        let stage = if result.state == "generated" {
            "emit"
        } else {
            "contract"
        };
        let file_fingerprints = result
            .paths
            .iter()
            .map(|path| {
                db.query_row(
                    "SELECT digest FROM files WHERE generation=?1 AND path=?2",
                    params![generation, path],
                    |row| row.get::<_, String>(0),
                )
            })
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let path_fingerprint = file_fingerprints.join("\n");
        let fingerprint = super::actions::fingerprint(
            stage,
            &[
                &result.address,
                &result.state,
                &result.reason,
                &result.paths.join("\n"),
                &path_fingerprint,
                &language,
                &instructions,
                &analyzer,
                &model,
                &prompt,
                &adapter,
            ],
        );
        super::actions::record(
            db,
            &super::actions::Record {
                generation,
                address: &result.address,
                stage,
                fingerprint: &fingerprint,
                state: &result.state,
                output: None,
                identity: super::actions::Identity {
                    model: &model,
                    prompt: &prompt,
                    adapter: &adapter,
                    validation: &validation,
                },
            },
        )?;
    }
    Ok(())
}
