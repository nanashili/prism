use super::database::Database;
use crate::error::{Result, failure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

pub(crate) struct EvidenceItem<'a> {
    pub address: &'a str,
    pub evidence: &'a Value,
}

impl Database {
    pub fn evidence_units(
        &self,
        generation: i64,
        lease: &str,
        offset: u64,
    ) -> Result<Vec<(String, Option<String>)>> {
        valid_lease(&self.db, generation, lease)?;
        let mut query = self.db.prepare(
            "SELECT address,evidence FROM units WHERE generation=?1 AND lease=?2 AND state='leased'
             AND address<>'__project__' ORDER BY address LIMIT 4 OFFSET ?3",
        )?;
        let rows = query.query_map(params![generation, lease, i64::try_from(offset)?], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn cached_evidence(&self, digest: &str) -> Result<Value> {
        Ok(serde_json::from_slice(&super::objects::get(
            &self.db, digest,
        )?)?)
    }

    pub fn save_evidence_batch(
        &mut self,
        generation: i64,
        lease: &str,
        items: &[EvidenceItem<'_>],
    ) -> Result<()> {
        let tx = self.db.transaction()?;
        valid_lease(&tx, generation, lease)?;
        let identity = tx.query_row(
            "SELECT analyzer,model,prompt_version,adapter_version,validation_version
             FROM generations WHERE id=?1",
            [generation],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;
        for item in items {
            let bytes = serde_json::to_vec(item.evidence)?;
            let evidence_text = std::str::from_utf8(&bytes)?;
            let digest = super::objects::put(&tx, &bytes)?;
            let changed = tx.execute(
                "UPDATE units SET evidence=?1 WHERE generation=?2 AND address=?3 AND lease=?4 AND state='leased'",
                params![digest, generation, item.address, lease],
            )?;
            if changed != 1 {
                return Err(failure("The batch no longer owns this function"));
            }
            let fingerprint = super::actions::fingerprint(
                "evidence",
                &[item.address, evidence_text, &identity.0],
            );
            super::actions::record(
                &tx,
                &super::actions::Record {
                    generation,
                    address: item.address,
                    stage: "evidence",
                    fingerprint: &fingerprint,
                    state: "complete",
                    output: Some(&digest),
                    identity: super::actions::Identity {
                        model: &identity.1,
                        prompt: &identity.2,
                        adapter: &identity.3,
                        validation: &identity.4,
                    },
                },
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn valid_lease(db: &Connection, generation: i64, lease: &str) -> Result<()> {
    super::queue::active(db, generation)?;
    let valid: Option<bool> = db.query_row(
        "SELECT state='leased' AND expires>unixepoch() FROM batches WHERE generation=?1 AND token=?2",
        params![generation,lease], |row| row.get(0),
    ).optional()?;
    if valid != Some(true) {
        return Err(failure("This batch expired or was released"));
    }
    Ok(())
}
