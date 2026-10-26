use crate::error::Result;
use rusqlite::{Connection, params};

pub(crate) fn initialize(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS verification_contexts(
             project TEXT PRIMARY KEY, identity TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS verification_evidence(
             id TEXT PRIMARY KEY, project TEXT NOT NULL, source TEXT NOT NULL,
             subject TEXT NOT NULL, kind TEXT NOT NULL, digest TEXT NOT NULL,
             data TEXT NOT NULL, current INTEGER NOT NULL, captured INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS verification_evidence_source
             ON verification_evidence(project,source,current);
         CREATE TABLE IF NOT EXISTS verification_claims(
             id INTEGER PRIMARY KEY, project TEXT NOT NULL, draft TEXT NOT NULL,
             assessment TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0,
             created INTEGER NOT NULL, updated INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS verification_claims_project
             ON verification_claims(project,id);
         CREATE TABLE IF NOT EXISTS verification_citations(
             claim INTEGER NOT NULL, evidence TEXT NOT NULL,
             PRIMARY KEY(claim,evidence));
         CREATE INDEX IF NOT EXISTS verification_citations_evidence
             ON verification_citations(evidence,claim);
         CREATE TABLE IF NOT EXISTS verification_history(
             claim INTEGER NOT NULL, revision INTEGER NOT NULL,
             draft TEXT NOT NULL, assessment TEXT NOT NULL, recorded INTEGER NOT NULL,
             PRIMARY KEY(claim,revision));
         CREATE TRIGGER IF NOT EXISTS verification_evidence_changed
         AFTER UPDATE OF current ON verification_evidence
         WHEN old.current=1 AND new.current=0 BEGIN
             UPDATE verification_claims
             SET assessment=json_object(
                 'status','unresolved','confidence','none',
                 'reason','Supporting analysis changed; capture fresh evidence and revise the claim',
                 'checks',json('[]')),
                 revision=revision+1, updated=unixepoch()
             WHERE id IN (SELECT claim FROM verification_citations WHERE evidence=old.id);
         END;
         CREATE TRIGGER IF NOT EXISTS verification_claim_history
         AFTER UPDATE OF revision ON verification_claims BEGIN
             INSERT INTO verification_history VALUES(
                 new.id,new.revision,new.draft,new.assessment,unixepoch());
         END;
         CREATE TRIGGER IF NOT EXISTS verification_decompilation_changed
         AFTER UPDATE OF digest,status ON decompilation_results
         WHEN old.digest IS NOT new.digest OR old.status IS NOT new.status BEGIN
             UPDATE verification_evidence SET current=0
             WHERE current=1 AND subject=new.address
               AND project=(SELECT project FROM decompilation_jobs WHERE id=new.job);
         END;",
    )?;
    Ok(())
}

/// Call before an external mutation: even failed analysis may have changed the provider.
fn invalidate(db: &Connection, project: &str) -> Result<()> {
    db.execute(
        "UPDATE verification_evidence SET current=0 WHERE project=?1 AND current=1",
        [project],
    )?;
    Ok(())
}

pub(crate) fn observe_pseudocode(
    db: &Connection,
    project: &str,
    address: &str,
    text: &str,
) -> Result<()> {
    let source = format!("view:Pseudocode:{address}");
    let unchanged: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM verification_evidence
         WHERE project=?1 AND source=?2 AND current=1 AND json_extract(data,'$.text')=?3)",
        params![project, source, text],
        |row| row.get(0),
    )?;
    if !unchanged {
        db.execute(
            "UPDATE verification_evidence SET current=0
             WHERE project=?1 AND subject=?2 AND current=1",
            params![project, address],
        )?;
    }
    Ok(())
}

impl crate::storage::Store {
    pub(crate) fn invalidate_verification(&self, project: &str) -> Result<()> {
        invalidate(&self.db, project)
    }

    pub(crate) fn invalidate_runtime_evidence(&self, project: &str) -> Result<()> {
        self.db.execute(
            "UPDATE verification_evidence SET current=0
             WHERE project=?1 AND kind=?2 AND current=1",
            params![
                project,
                serde_json::to_string(&super::model::SourceKind::Runtime)?
            ],
        )?;
        Ok(())
    }

    pub(crate) fn verification_context(&self, project: &str, identity: &str) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        let changed: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM verification_contexts
             WHERE project=?1 AND identity<>?2)",
            params![project, identity],
            |row| row.get(0),
        )?;
        if changed {
            invalidate(&tx, project)?;
        }
        tx.execute(
            "INSERT INTO verification_contexts VALUES(?1,?2)
             ON CONFLICT(project) DO UPDATE SET identity=excluded.identity",
            params![project, identity],
        )?;
        tx.commit()?;
        Ok(())
    }
}
