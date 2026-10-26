use super::model::{Assessment, Claim, ClaimDraft};
use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

impl Store {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(crate) fn propose_claim(&self, project: &str, draft: &ClaimDraft) -> Result<Value> {
        draft.validate()?;
        let tx = self.db.unchecked_transaction()?;
        // Validate ownership and integrity before any claim becomes visible.
        validate_evidence(&tx, project, draft)?;
        let assessment = serde_json::to_string(&Assessment::proposed())?;
        tx.execute(
            "INSERT INTO verification_claims(project,draft,assessment,created,updated)
             VALUES(?1,?2,?3,unixepoch(),unixepoch())",
            params![project, serde_json::to_string(draft)?, assessment],
        )?;
        let id = tx.last_insert_rowid();
        for check in &draft.checks {
            tx.execute(
                "INSERT OR IGNORE INTO verification_citations VALUES(?1,?2)",
                params![id, check.evidence],
            )?;
        }
        tx.execute(
            "INSERT INTO verification_history VALUES(?1,0,?2,?3,unixepoch())",
            params![id, serde_json::to_string(draft)?, assessment],
        )?;
        let claim = load(&tx, project, id)?;
        tx.commit()?;
        with_citation(claim)
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(crate) fn revise_claim(&self, project: &str, id: i64, draft: &ClaimDraft) -> Result<Value> {
        draft.validate()?;
        let tx = self.db.unchecked_transaction()?;
        load(&tx, project, id)?;
        validate_evidence(&tx, project, draft)?;
        tx.execute(
            "UPDATE verification_claims SET draft=?3,assessment=?4,revision=revision+1,updated=unixepoch()
             WHERE project=?1 AND id=?2",
            params![project, id, serde_json::to_string(draft)?, serde_json::to_string(&Assessment::proposed())?],
        )?;
        tx.execute("DELETE FROM verification_citations WHERE claim=?1", [id])?;
        for check in &draft.checks {
            tx.execute(
                "INSERT OR IGNORE INTO verification_citations VALUES(?1,?2)",
                params![id, check.evidence],
            )?;
        }
        let claim = load(&tx, project, id)?;
        tx.commit()?;
        with_citation(claim)
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(crate) fn verify_claim(&self, project: &str, id: i64) -> Result<Value> {
        let tx = self.db.unchecked_transaction()?;
        let claim = load(&tx, project, id)?;
        let evidence = claim
            .draft
            .checks
            .iter()
            .map(|check| super::evidence::load(&tx, project, &check.evidence))
            .collect::<Result<Vec<_>>>();
        let assessment = match evidence {
            Ok(evidence) => super::assessment::assess(&claim.draft, &evidence),
            Err(error) => Assessment {
                status: super::model::ClaimStatus::Unresolved,
                confidence: super::model::Confidence::None,
                reason: format!("Evidence could not be verified: {error}"),
                checks: Vec::new(),
            },
        };
        tx.execute(
            "UPDATE verification_claims SET assessment=?3,revision=revision+1,updated=unixepoch()
             WHERE project=?1 AND id=?2",
            params![project, id, serde_json::to_string(&assessment)?],
        )?;
        let claim = load(&tx, project, id)?;
        tx.commit()?;
        with_citation(claim)
    }

    pub(crate) fn verification_claim(&self, project: &str, id: i64) -> Result<Value> {
        let claim = load(&self.db, project, id)?;
        let mut result = with_citation(claim)?;
        let mut statement = self.db.prepare(
            "SELECT revision,assessment,recorded,draft FROM verification_history
             WHERE claim=?1 ORDER BY revision DESC LIMIT 8",
        )?;
        let rows = statement.query_map([id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut history = Vec::new();
        for row in rows {
            let (revision, assessment, recorded, draft) = row?;
            let assessment: Value = serde_json::from_str(&assessment)?;
            let draft: Value = serde_json::from_str(&draft)?;
            history.push(json!({"revision":revision,"draft":draft,"assessment":assessment,"recorded":recorded}));
        }
        result["history"] = history.into();
        Ok(result)
    }

    pub(crate) fn verification_claims(&self, project: &str, offset: u64) -> Result<Value> {
        let mut statement = self.db.prepare(
            "SELECT id FROM verification_claims WHERE project=?1 ORDER BY id LIMIT 17 OFFSET ?2",
        )?;
        let ids = statement
            .query_map(params![project, i64::try_from(offset)?], |row| {
                row.get::<_, i64>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let claims = ids
            .iter()
            .take(16)
            .map(|id| with_citation(load(&self.db, project, *id)?))
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"claims":claims,"next_offset":(ids.len()>16).then(|| offset.saturating_add(16))}))
    }
}

fn validate_evidence(db: &Connection, project: &str, draft: &ClaimDraft) -> Result<()> {
    for check in &draft.checks {
        let evidence = super::evidence::load(db, project, &check.evidence)?;
        if !evidence.current {
            return Err(failure(
                "Cannot cite stale evidence; capture fresh analysis",
            ));
        }
    }
    Ok(())
}

fn load(db: &Connection, project: &str, id: i64) -> Result<Claim> {
    let (draft, assessment, revision, created, updated): (String, String, i64, i64, i64) = db
        .query_row(
            "SELECT draft,assessment,revision,created,updated FROM verification_claims
         WHERE project=?1 AND id=?2",
            params![project, id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
    Ok(Claim {
        id,
        project: project.into(),
        draft: serde_json::from_str(&draft)?,
        assessment: serde_json::from_str(&assessment)?,
        revision,
        created,
        updated,
    })
}

fn with_citation(claim: Claim) -> Result<Value> {
    let citation = format!("[claim:{}]", claim.id);
    let mut value = serde_json::to_value(claim)?;
    value["citation"] = citation.into();
    Ok(value)
}
