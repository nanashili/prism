use super::database::Database;
use crate::error::{Result, failure};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

pub(crate) struct Configuration<'a> {
    pub language: &'a str,
    pub instructions: &'a str,
    pub analyzer: &'a str,
    pub model: &'a str,
    pub prompt_version: &'a str,
    pub adapter_version: &'a str,
    pub validation_version: &'a str,
    pub batch_size: u32,
    pub workers: u32,
}

impl Database {
    pub fn start(
        &mut self,
        store: &crate::storage::Store,
        project: &str,
        config: &Configuration<'_>,
        fork: bool,
    ) -> Result<i64> {
        validate(config)?;
        self.verify_project(project)?;
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                3,
                project,
                config.language,
                config.instructions,
                config.analyzer,
                config.model,
                config.prompt_version,
                config.adapter_version,
                config.validation_version,
            ))?)
        );
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous = tx
            .query_row(
                "SELECT id,config,analyzer,language,instructions,model,prompt_version,
                        adapter_version,validation_version
                 FROM generations ORDER BY id DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                },
            )
            .optional()?;
        if let Some((id, key, ..)) = &previous {
            if !fork {
                resume_generation(&tx, *id, key, &fingerprint, config)?;
                tx.commit()?;
                return Ok(*id);
            }
            super::queue::expire(&tx, *id)?;
            ensure_batches_finished(&tx, *id)?;
        }
        tx.execute(
            "INSERT INTO generations(parent,project,language,instructions,config,analyzer,
             model,prompt_version,adapter_version,validation_version,batch_size,workers)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                previous.as_ref().map(|(id, ..)| id),
                project,
                config.language,
                config.instructions,
                fingerprint,
                config.analyzer,
                config.model,
                config.prompt_version,
                config.adapter_version,
                config.validation_version,
                config.batch_size,
                config.workers
            ],
        )?;
        let id = tx.last_insert_rowid();
        if let Some((
            parent,
            _key,
            analyzer,
            language,
            instructions,
            model,
            prompt,
            adapter,
            _validation,
        )) = previous
        {
            let evidence_reuse = analyzer == config.analyzer;
            let source_reuse = evidence_reuse
                && config.model != "default"
                && language == config.language
                && instructions == config.instructions
                && model == config.model
                && prompt == config.prompt_version
                && adapter == config.adapter_version;
            super::generation_copy::copy(&tx, parent, id, source_reuse, evidence_reuse)?;
            tx.execute(
                "UPDATE generations SET state='frozen' WHERE id=?1",
                [parent],
            )?;
        } else {
            store.reconstruction_inventory(&tx, project, id)?;
        }
        tx.commit()?;
        Ok(id)
    }
}

fn resume_generation(
    db: &rusqlite::Connection,
    id: i64,
    previous: &str,
    fingerprint: &str,
    config: &Configuration<'_>,
) -> Result<()> {
    if previous != fingerprint {
        return Err(failure(
            "Settings changed. Start a new generation to keep the earlier output",
        ));
    }
    db.execute(
        "UPDATE generations SET state='active',batch_size=?2,workers=?3 WHERE id=?1",
        params![id, config.batch_size, config.workers],
    )?;
    Ok(())
}

fn ensure_batches_finished(db: &rusqlite::Connection, generation: i64) -> Result<()> {
    let active: i64 = db.query_row(
        "SELECT count(*) FROM batches WHERE generation=?1 AND state='leased'",
        [generation],
        |row| row.get(0),
    )?;
    if active > 0 {
        return Err(failure(
            "Release or finish active batches before starting a new generation",
        ));
    }
    Ok(())
}

fn validate(config: &Configuration<'_>) -> Result<()> {
    if config.language.trim().is_empty() || config.language.len() > 80 {
        return Err(failure("Choose a target language of up to 80 bytes"));
    }
    if config.analyzer.trim().is_empty()
        || config.model.trim().is_empty()
        || config.prompt_version.trim().is_empty()
        || config.adapter_version.trim().is_empty()
        || config.validation_version.trim().is_empty()
    {
        return Err(failure(
            "Analyzer, model, prompt, adapter and validation versions are required",
        ));
    }
    if config.instructions.len() > 32 * 1024
        || config.model.len() > 256
        || config.prompt_version.len() > 256
        || config.adapter_version.len() > 256
        || config.validation_version.len() > 256
        || [
            config.model,
            config.prompt_version,
            config.adapter_version,
            config.validation_version,
        ]
        .iter()
        .any(|value| value.contains('\0'))
        || !(1..=64).contains(&config.batch_size)
        || !(1..=8).contains(&config.workers)
    {
        return Err(failure(
            "Use 1–64 functions per batch, 1–8 workers and at most 32 KiB of instructions",
        ));
    }
    Ok(())
}
