use super::model::{Evidence, EvidenceSource, SourceKind};
use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Durable evidence is bounded independently from the evictable view cache.
const MAX_PROJECT_BYTES: i64 = 32 * 1024 * 1024;

struct PreparedSnapshot {
    data: String,
    digest: String,
    kind: String,
}

impl PreparedSnapshot {
    fn new(source: &EvidenceSource<'_>, data: &Value) -> Result<Self> {
        if source.key.len() > 32 * 1024 || source.subject.len() > 1024 {
            return Err(failure("Evidence provenance exceeds its storage limit"));
        }
        let data = serde_json::to_string(data)?;
        if data.len() > 512 * 1024 {
            return Err(failure(
                "Evidence exceeds 512 KiB; select a smaller analysis result",
            ));
        }
        Ok(Self {
            digest: format!("{:x}", Sha256::digest(data.as_bytes())),
            kind: serde_json::to_string(&source.kind)?,
            data,
        })
    }
}

impl Store {
    pub(crate) fn capture_evidence(
        &self,
        project: &str,
        source: &EvidenceSource<'_>,
        data: &Value,
    ) -> Result<Value> {
        let result = self.write_evidence(project, source, data);
        if result.is_err() {
            // The provider observation already happened. If persistence fails,
            // retain the old snapshot but stop treating it as current evidence.
            let cleanup = self
                .db
                .execute(
                    "UPDATE verification_evidence SET current=0
                 WHERE project=?1 AND current=1 AND (source=?2 OR (subject=?3 AND ?3<>''))",
                    params![project, source.key, source.subject],
                )
                .map(|_| ())
                .map_err(Into::into);
            return super::recovery::finish(result, cleanup, "Invalidating unrecorded evidence");
        }
        result
    }

    fn write_evidence(
        &self,
        project: &str,
        source: &EvidenceSource<'_>,
        data: &Value,
    ) -> Result<Value> {
        let snapshot = PreparedSnapshot::new(source, data)?;
        let tx = self.db.unchecked_transaction()?;
        let id = match current_snapshot(&tx, project, source.key, &snapshot.digest)? {
            Some(id) => id,
            None => replace_snapshot(&tx, project, source, &snapshot)?,
        };
        tx.commit()?;
        Ok(receipt(&id, &snapshot.digest))
    }

    pub(crate) fn verification_evidence(&self, project: &str, id: &str) -> Result<Value> {
        Ok(serde_json::to_value(load(&self.db, project, id)?)?)
    }

    pub(crate) fn latest_runtime_evidence(&self, project: &str) -> Result<Value> {
        let id: String = self.db.query_row(
            "SELECT id FROM verification_evidence
             WHERE project=?1 AND kind=?2 AND current=1 ORDER BY captured DESC,rowid DESC LIMIT 1",
            params![project, serde_json::to_string(&SourceKind::Runtime)?],
            |row| row.get(0),
        )?;
        self.verification_evidence(project, &id)
    }
}

fn current_snapshot(
    db: &Connection,
    project: &str,
    source: &str,
    digest: &str,
) -> Result<Option<String>> {
    use rusqlite::OptionalExtension;
    let id: Option<String> = db
        .query_row(
            "SELECT id FROM verification_evidence
         WHERE project=?1 AND source=?2 AND digest=?3 AND current=1",
            params![project, source, digest],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = &id {
        load(db, project, id)?;
    }
    Ok(id)
}

fn replace_snapshot(
    db: &Connection,
    project: &str,
    source: &EvidenceSource<'_>,
    snapshot: &PreparedSnapshot,
) -> Result<String> {
    let additional = snapshot.data.len() + source.key.len() + source.subject.len();
    enforce_budget(db, project, additional)?;
    let id = snapshot_id(project, source, &snapshot.digest)?;

    let replaced: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM verification_evidence
         WHERE project=?1 AND source=?2 AND current=1)",
        params![project, source.key],
        |row| row.get(0),
    )?;
    db.execute(
        "UPDATE verification_evidence SET current=0
         WHERE project=?1 AND current=1
           AND (source=?2 OR (?4 AND subject=?3 AND ?3<>''))",
        params![project, source.key, source.subject, replaced],
    )?;
    db.execute(
        "INSERT INTO verification_evidence VALUES(?1,?2,?3,?4,?5,?6,?7,1,unixepoch())",
        params![
            id,
            project,
            source.key,
            source.subject,
            snapshot.kind,
            snapshot.digest,
            snapshot.data
        ],
    )?;
    Ok(id)
}

fn snapshot_id(project: &str, source: &EvidenceSource<'_>, digest: &str) -> Result<String> {
    // Historical snapshots remain immutable, including after the source returns
    // to identical content. A new capture must never revive stale citations.
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|error| failure(error.to_string()))?;
    let identity = serde_json::to_vec(&(
        project,
        source.key,
        source.subject,
        source.kind,
        digest,
        nonce,
    ))?;
    Ok(format!("{:x}", Sha256::digest(identity)))
}

pub(super) fn load(db: &Connection, project: &str, id: &str) -> Result<Evidence> {
    let (source, subject, kind, digest, data, current, captured): (
        String,
        String,
        String,
        String,
        String,
        bool,
        i64,
    ) = db.query_row(
        "SELECT source,subject,kind,digest,data,current,captured
         FROM verification_evidence WHERE project=?1 AND id=?2",
        params![project, id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    if digest != format!("{:x}", Sha256::digest(data.as_bytes())) {
        return Err(failure(format!("Evidence {id} failed its digest check")));
    }
    Ok(Evidence {
        id: id.into(),
        source,
        subject,
        kind: serde_json::from_str(&kind)?,
        digest,
        data: serde_json::from_str(&data)?,
        current,
        captured,
    })
}

fn enforce_budget(db: &Connection, project: &str, additional: usize) -> Result<()> {
    let used: i64 = db.query_row(
        "SELECT coalesce(sum(length(CAST(data AS BLOB))
             + length(CAST(source AS BLOB)) + length(CAST(subject AS BLOB))),0)
         FROM verification_evidence WHERE project=?1",
        [project],
        |row| row.get(0),
    )?;
    if used.saturating_add(i64::try_from(additional)?) > MAX_PROJECT_BYTES {
        return Err(failure(
            "The project's 32 MiB evidence budget is full; historical evidence was retained",
        ));
    }
    Ok(())
}

fn receipt(id: &str, digest: &str) -> Value {
    json!({"id":id,"digest":digest,"citation":format!("[evidence:{id}]"),"current":true})
}
