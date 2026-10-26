#[cfg(test)]
mod tests;

use super::Store;
use crate::analysis::type_recovery::{Evidence, Model, infer};
use crate::error::{Result, failure};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Propose {
        evidence: Evidence,
        #[serde(default)]
        update: Update,
    },
    Check {
        proposal: String,
    },
    Accept {
        proposal: String,
    },
    Read {
        address: Option<String>,
    },
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Update {
    #[default]
    Merge,
    Replace,
}

struct Proposal {
    revision: i64,
    evidence: Evidence,
}

pub(super) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS recovered_types(
            project TEXT NOT NULL, analyzer TEXT NOT NULL, revision INTEGER NOT NULL,
            evidence TEXT NOT NULL, model TEXT NOT NULL,
            PRIMARY KEY(project,analyzer));
         CREATE TABLE IF NOT EXISTS type_proposals(
            project TEXT NOT NULL, analyzer TEXT NOT NULL, id TEXT NOT NULL,
            base_revision INTEGER NOT NULL, evidence TEXT NOT NULL,
            PRIMARY KEY(project,analyzer));",
    )?;
    Ok(())
}

impl Store {
    pub(crate) fn recover_types(
        &mut self,
        project: &str,
        analyzer: &str,
        args: &Value,
    ) -> Result<Value> {
        if serde_json::to_vec(args)?.len() > 400 * 1024 {
            return Err(failure("The type-recovery request exceeds 400 KiB"));
        }
        let request: Request = serde_json::from_value(args.clone())?;
        match request {
            Request::Propose { evidence, update } => {
                self.propose_types(project, analyzer, evidence, update)
            }
            Request::Check { proposal } => self.check_types(project, analyzer, &proposal),
            Request::Accept { proposal } => self.accept_types(project, analyzer, &proposal),
            Request::Read { address } => {
                let addresses: Vec<&str> = address.iter().map(String::as_str).collect();
                self.read_types(project, analyzer, &addresses)
            }
        }
    }

    fn read_types(&self, project: &str, analyzer: &str, addresses: &[&str]) -> Result<Value> {
        // One query keeps editable observations and their model on the same revision,
        // even if another window accepts a proposal concurrently.
        let row: Option<(i64, String, String)> = self.db.query_row(
            "SELECT revision,model,evidence FROM recovered_types WHERE project=?1 AND analyzer=?2",
            params![project, analyzer], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional()?;
        let Some((revision, model, evidence)) = row else {
            let mut reply = context_reply(0, &json!({"types":[],"prototypes":[]}));
            reply["observations"] = Value::Null;
            return Ok(reply);
        };
        let mut model: Value = serde_json::from_str(&model)?;
        filter_context(&mut model, addresses);
        let mut reply = context_reply(revision, &model);
        reply["observations"] = serde_json::from_str(&evidence)?;
        Ok(reply)
    }

    fn propose_types(
        &mut self,
        project: &str,
        analyzer: &str,
        incoming: Evidence,
        update: Update,
    ) -> Result<Value> {
        incoming.validate()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proposal = prepare_proposal(&tx, project, analyzer, incoming, update)?;
        let model = checked_model(&tx, project, &proposal.evidence)?;
        let encoded = serde_json::to_string(&proposal.evidence)?;
        let id = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                project,
                analyzer,
                proposal.revision,
                &proposal.evidence
            ))?)
        );

        tx.execute(
            "INSERT INTO type_proposals VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(project,analyzer) DO UPDATE SET
                id=excluded.id,base_revision=excluded.base_revision,evidence=excluded.evidence",
            params![project, analyzer, id, proposal.revision, encoded],
        )?;
        tx.commit()?;
        Ok(proposal_reply(&id, proposal.revision, &model))
    }

    fn check_types(&mut self, project: &str, analyzer: &str, id: &str) -> Result<Value> {
        let tx = self.db.transaction()?;
        let proposal = load_proposal(&tx, project, analyzer, id)?;
        let model = checked_model(&tx, project, &proposal.evidence)?;
        let reply = proposal_reply(id, proposal.revision, &model);
        tx.commit()?;
        Ok(reply)
    }

    fn accept_types(&mut self, project: &str, analyzer: &str, id: &str) -> Result<Value> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proposal = load_proposal(&tx, project, analyzer, id)?;
        let model = checked_model(&tx, project, &proposal.evidence)?;
        if !model.conflicts.is_empty() {
            return Err(failure(
                "Type consistency checks failed; inspect the proposal's conflicts before accepting",
            ));
        }
        let revision = proposal
            .revision
            .checked_add(1)
            .ok_or("Type revision overflow")?;
        tx.execute(
            "INSERT INTO recovered_types VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(project,analyzer) DO UPDATE SET
                revision=excluded.revision,evidence=excluded.evidence,model=excluded.model",
            params![
                project,
                analyzer,
                revision,
                serde_json::to_string(&proposal.evidence)?,
                serde_json::to_string(&model)?
            ],
        )?;
        tx.execute(
            "DELETE FROM type_proposals WHERE project=?1 AND analyzer=?2",
            params![project, analyzer],
        )?;
        tx.commit()?;
        Ok(json!({"accepted":true,"revision":revision,"model":model,
            "scope":"agent_evidence","provider_types_applied":false,"behavior_verified":false}))
    }

    /// Supplies one shared model for the whole result page, including pointee dependencies.
    pub(crate) fn recovered_type_context(
        &self,
        project: &str,
        analyzer: &str,
        addresses: &[&str],
    ) -> Result<Option<Value>> {
        let row: Option<(i64, String)> = self
            .db
            .query_row(
                "SELECT revision,model FROM recovered_types WHERE project=?1 AND analyzer=?2",
                params![project, analyzer],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((revision, encoded)) = row else {
            return Ok(None);
        };
        let mut model: Value = serde_json::from_str(&encoded)?;
        filter_context(&mut model, addresses);
        Ok(Some(context_reply(revision, &model)))
    }
}

fn prepare_proposal(
    tx: &Transaction<'_>,
    project: &str,
    analyzer: &str,
    incoming: Evidence,
    update: Update,
) -> Result<Proposal> {
    let accepted: Option<(i64, String)> = tx
        .query_row(
            "SELECT revision,evidence FROM recovered_types WHERE project=?1 AND analyzer=?2",
            params![project, analyzer],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((revision, encoded)) = accepted else {
        return Ok(Proposal {
            revision: 0,
            evidence: incoming,
        });
    };

    let evidence = match update {
        Update::Merge => {
            let mut evidence: Evidence = serde_json::from_str(&encoded)?;
            evidence.merge(incoming)?;
            evidence
        }
        Update::Replace => incoming,
    };
    Ok(Proposal { revision, evidence })
}

fn checked_model(tx: &Transaction<'_>, project: &str, evidence: &Evidence) -> Result<Model> {
    let mut model = infer(evidence)?;
    let mut query = tx
        .prepare_cached("SELECT EXISTS(SELECT 1 FROM functions WHERE project=?1 AND address=?2)")?;
    for function in evidence.functions() {
        let exists: bool = query.query_row(params![project, function], |row| row.get(0))?;
        if !exists {
            model.conflicts.push(format!(
                "{function}: function is absent from the saved inventory"
            ));
        }
    }
    if serde_json::to_vec(&model)?.len() > 768 * 1024 {
        return Err(failure(
            "The recovered model exceeds 768 KiB; use fewer observations",
        ));
    }
    Ok(model)
}

fn load_proposal(
    tx: &Transaction<'_>,
    project: &str,
    analyzer: &str,
    id: &str,
) -> Result<Proposal> {
    if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(failure("A proposal id must be a SHA-256 identifier"));
    }
    let row: Option<(i64, String)> = tx.query_row(
        "SELECT base_revision,evidence FROM type_proposals WHERE project=?1 AND analyzer=?2 AND id=?3",
        params![project, analyzer, id], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?;
    let (revision, encoded) = row.ok_or(
        "The proposal is missing, superseded, or belongs to another analysis configuration",
    )?;
    let current: i64 = tx.query_row(
        "SELECT coalesce((SELECT revision FROM recovered_types WHERE project=?1 AND analyzer=?2),0)",
        params![project, analyzer], |row| row.get(0),
    )?;
    if current != revision {
        return Err(failure(
            "The accepted types changed; propose again before accepting",
        ));
    }
    Ok(Proposal {
        revision,
        evidence: serde_json::from_str(&encoded)?,
    })
}

fn proposal_reply(id: &str, revision: i64, model: &Model) -> Value {
    json!({"proposal":id,"base_revision":revision,"consistent":model.conflicts.is_empty(),
        "accepted":false,"model":model,"scope":"agent_evidence",
        "provider_types_applied":false,"behavior_verified":false})
}

fn context_reply(revision: i64, model: &Value) -> Value {
    json!({"revision":revision,"types":model["types"],"prototypes":model["prototypes"],
        "scope":"agent_evidence","provider_types_applied":false,"behavior_verified":false})
}

fn filter_context(model: &mut Value, addresses: &[&str]) {
    if addresses.is_empty() {
        return;
    }
    let mut needed = BTreeSet::new();
    select_prototypes(model, addresses, &mut needed);
    let Some(types) = model["types"].as_array_mut() else {
        return;
    };
    select_consumers(types, addresses, &mut needed);
    include_pointees(types, &mut needed);
    types.retain(|item| item["id"].as_str().is_some_and(|id| needed.contains(id)));
}

fn select_prototypes(model: &mut Value, addresses: &[&str], needed: &mut BTreeSet<String>) {
    if let Some(prototypes) = model["prototypes"].as_array_mut() {
        prototypes
            .retain(|item| addresses.contains(&item["function"].as_str().unwrap_or_default()));
        for prototype in prototypes {
            for value in prototype["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .chain(std::iter::once(&prototype["returns"]))
            {
                if let Some(id) = value["pointee"].as_str() {
                    needed.insert(id.to_owned());
                }
            }
        }
    }
}

fn select_consumers(types: &[Value], addresses: &[&str], needed: &mut BTreeSet<String>) {
    for recovered in types.iter() {
        if recovered["consumers"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|base| addresses.contains(&base["function"].as_str().unwrap_or_default()))
            && let Some(id) = recovered["id"].as_str()
        {
            needed.insert(id.to_owned());
        }
    }
}

fn include_pointees(types: &[Value], needed: &mut BTreeSet<String>) {
    loop {
        let before = needed.len();
        for recovered in types.iter() {
            if recovered["id"]
                .as_str()
                .is_some_and(|id| needed.contains(id))
            {
                for field in recovered["fields"].as_array().into_iter().flatten() {
                    if let Some(id) = field["pointee"].as_str() {
                        needed.insert(id.to_owned());
                    }
                }
            }
        }
        if needed.len() == before {
            break;
        }
    }
}
