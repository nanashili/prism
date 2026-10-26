use super::database::Database;
use crate::error::{Result, failure};
use crate::reconstruction::{EvidenceReuse, SourceReuse};
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
        mode: GenerationStart,
    ) -> Result<i64> {
        validate(config)?;
        self.verify_project(project)?;
        let fingerprint = config.fingerprint(project)?;

        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous = PreviousGeneration::load(&tx)?;
        if let Some(previous) = &previous {
            match mode {
                GenerationStart::Resume => {
                    resume_generation(
                        &tx,
                        previous.id,
                        &previous.fingerprint,
                        &fingerprint,
                        config,
                    )?;
                    tx.commit()?;
                    return Ok(previous.id);
                }
                GenerationStart::Fork => previous.prepare_fork(&tx)?,
            }
        }

        let id = insert_generation(&tx, previous.as_ref(), project, &fingerprint, config)?;
        if let Some(previous) = previous {
            previous.copy_outputs(&tx, id, config)?;
        } else {
            store.reconstruction_inventory(&tx, project, id)?;
        }
        tx.commit()?;
        Ok(id)
    }
}

impl Configuration<'_> {
    fn fingerprint(&self, project: &str) -> Result<String> {
        let identity = serde_json::to_vec(&(
            3,
            project,
            self.language,
            self.instructions,
            self.analyzer,
            self.model,
            self.prompt_version,
            self.adapter_version,
            self.validation_version,
        ))?;
        Ok(format!("{:x}", Sha256::digest(identity)))
    }
}

struct PreviousGeneration {
    id: i64,
    fingerprint: String,
    analyzer: String,
    language: String,
    instructions: String,
    model: String,
    prompt: String,
    adapter: String,
}

impl PreviousGeneration {
    fn load(db: &rusqlite::Connection) -> Result<Option<Self>> {
        Ok(db.query_row(
            "SELECT id,config,analyzer,language,instructions,model,prompt_version,adapter_version
             FROM generations ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok(Self {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                analyzer: row.get(2)?,
                language: row.get(3)?,
                instructions: row.get(4)?,
                model: row.get(5)?,
                prompt: row.get(6)?,
                adapter: row.get(7)?,
            }),
        ).optional()?)
    }

    fn prepare_fork(&self, db: &rusqlite::Connection) -> Result<()> {
        super::queue::expire(db, self.id)?;
        ensure_batches_finished(db, self.id)
    }

    fn copy_outputs(
        &self,
        db: &rusqlite::Connection,
        child: i64,
        config: &Configuration<'_>,
    ) -> Result<()> {
        let evidence_reuse = self.analyzer == config.analyzer;
        let source_reuse = evidence_reuse
            && config.model != "default"
            && self.language == config.language
            && self.instructions == config.instructions
            && self.model == config.model
            && self.prompt == config.prompt_version
            && self.adapter == config.adapter_version;
        super::generation_copy::copy(
            db,
            self.id,
            child,
            SourceReuse::from(source_reuse),
            EvidenceReuse::from(evidence_reuse),
        )?;
        db.execute(
            "UPDATE generations SET state='frozen' WHERE id=?1",
            [self.id],
        )?;
        Ok(())
    }
}

fn insert_generation(
    db: &rusqlite::Connection,
    previous: Option<&PreviousGeneration>,
    project: &str,
    fingerprint: &str,
    config: &Configuration<'_>,
) -> Result<i64> {
    db.execute(
        "INSERT INTO generations(parent,project,language,instructions,config,analyzer,
         model,prompt_version,adapter_version,validation_version,batch_size,workers)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            previous.map(|generation| generation.id),
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
            config.workers,
        ],
    )?;
    Ok(db.last_insert_rowid())
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GenerationStart {
    Fork,
    Resume,
}

impl From<bool> for GenerationStart {
    fn from(value: bool) -> Self {
        if value { Self::Fork } else { Self::Resume }
    }
}

#[cfg(test)]
mod tests {
    use super::{Configuration, GenerationStart};
    use crate::error::Result;
    use crate::reconstruction::{Database, DatabaseOpen};
    use crate::storage::Store;

    fn configuration() -> Configuration<'static> {
        Configuration {
            language: "rust",
            instructions: "",
            analyzer: "analyzer-v1",
            model: "selected-model",
            prompt_version: "prompt-v1",
            adapter_version: "adapter-v1",
            validation_version: "validation-v1",
            batch_size: 4,
            workers: 1,
        }
    }

    #[test]
    fn resume_preserves_the_generation_and_rejects_changed_identity() -> Result<()> {
        let folder = tempfile::tempdir()?;
        let store = Store::open(&folder.path().join("profile.sqlite"))?;
        let mut database = Database::open(folder.path(), DatabaseOpen::Create)?;
        let mut config = configuration();
        let original = database.start(&store, "project", &config, GenerationStart::Resume)?;

        config.workers = 2;
        let resumed = database.start(&store, "project", &config, GenerationStart::Resume)?;
        assert_eq!(
            resumed, original,
            "scheduling changes must resume existing work"
        );

        config.analyzer = "analyzer-v2";
        database
            .start(&store, "project", &config, GenerationStart::Resume)
            .expect_err("analyzer changes must require a new generation");
        assert_eq!(
            database.generation(None)?,
            original,
            "a rejected resume must not create a generation"
        );
        let workers: u32 = database.db.query_row(
            "SELECT workers FROM generations WHERE id=?1",
            [original],
            |row| row.get(0),
        )?;
        assert_eq!(
            workers, 2,
            "the successful resume must retain its scheduling update"
        );
        Ok(())
    }

    #[test]
    fn failed_fork_preserves_the_parent_and_can_be_retried() -> Result<()> {
        let folder = tempfile::tempdir()?;
        let store = Store::open(&folder.path().join("profile.sqlite"))?;
        let mut database = Database::open(folder.path(), DatabaseOpen::Create)?;
        let config = configuration();
        let parent = database.start(&store, "project", &config, GenerationStart::Resume)?;
        database.db.execute_batch(
            "UPDATE units SET state='generated',evidence='evidence-v1';
             CREATE TRIGGER reject_unit_copy BEFORE INSERT ON units
             WHEN NEW.generation > 1 BEGIN SELECT RAISE(ABORT,'simulated copy failure'); END;",
        )?;

        database
            .start(&store, "project", &config, GenerationStart::Fork)
            .expect_err("a failed copy must abort the fork");
        assert_eq!(
            database.generation(None)?,
            parent,
            "the failed transaction must not publish a child"
        );
        let state: String = database.db.query_row(
            "SELECT state FROM generations WHERE id=?1",
            [parent],
            |row| row.get(0),
        )?;
        assert_eq!(state, "active", "a failed fork must not freeze its parent");

        database
            .db
            .execute_batch("DROP TRIGGER reject_unit_copy;")?;
        let child = database.start(&store, "project", &config, GenerationStart::Fork)?;
        let copied: (String, String) = database.db.query_row(
            "SELECT state,evidence FROM units WHERE generation=?1 AND address='__project__'",
            [child],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(
            copied,
            ("reused".into(), "evidence-v1".into()),
            "retry must copy reusable output and evidence"
        );
        Ok(())
    }
}
