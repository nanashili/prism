use super::{database::Database, modules};
use crate::error::{Result, failure};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MAX_CONTRACTS: usize = 32;
const MAX_CONTENT: usize = 8_192;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    module: String,
    interface: String,
    abi: String,
    #[serde(default)]
    unknowns: String,
    #[serde(default)]
    ready: bool,
}

impl Database {
    pub fn set_contracts(&mut self, generation: i64, payload: &Value) -> Result<Value> {
        let raw = payload
            .get("contracts")
            .and_then(Value::as_array)
            .ok_or("Supply a contracts array")?;
        if raw.len() > MAX_CONTRACTS {
            return Err(failure("A contract update is limited to 32 modules"));
        }
        let contracts: Vec<Contract> = serde_json::from_value(Value::Array(raw.clone()))?;
        if contracts.len() > MAX_CONTRACTS {
            return Err(failure("A contract update is limited to 32 modules"));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        super::queue::active(&tx, generation)?;
        modules::refuse_leased(&tx, generation)?;
        let mut names = BTreeSet::new();
        let mut changed = Vec::new();
        for contract in &contracts {
            validate(&tx, generation, contract)?;
            if !names.insert(contract.module.clone()) {
                return Err(failure(
                    "A contract update cannot contain duplicate modules",
                ));
            }
            let interface_hash = digest(&contract.interface);
            let abi_hash = digest(&contract.abi);
            let old: Option<(String, String, String, i64)> = tx
                .query_row(
                    "SELECT interface_hash,abi_hash,unknowns,ready FROM contracts
                 WHERE generation=?1 AND module=?2",
                    params![generation, contract.module],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()?;
            let next_ready =
                contract.ready && !contract.interface.is_empty() && !contract.abi.is_empty();
            if old.as_ref().is_none_or(|value| {
                value.0 != interface_hash
                    || value.1 != abi_hash
                    || value.2 != contract.unknowns
                    || value.3 != next_ready as i64
            }) {
                changed.push(contract.module.clone());
            }
            tx.execute(
                "INSERT INTO contracts(generation,module,interface,abi,interface_hash,abi_hash,unknowns,ready)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(generation,module) DO UPDATE SET interface_hash=excluded.interface_hash,
                 abi_hash=excluded.abi_hash,interface=excluded.interface,abi=excluded.abi,
                 unknowns=excluded.unknowns,ready=excluded.ready",
                params![generation, contract.module, contract.interface, contract.abi,
                    interface_hash, abi_hash, contract.unknowns, next_ready as i64],
            )?;
            tx.execute(
                "UPDATE modules SET contract_ready=?1 WHERE generation=?2 AND module=?3",
                params![next_ready as i64, generation, contract.module],
            )?;
        }
        modules::invalidate_consumers(&tx, generation, &changed)?;
        if !changed.is_empty() {
            tx.execute(
                "UPDATE generations SET revision=revision+1 WHERE id=?1",
                [generation],
            )?;
            tx.execute("DELETE FROM actions WHERE generation=?1 AND address='__generation__' AND stage='validate'", [generation])?;
        }
        tx.commit()?;
        Ok(json!({"saved": true, "contracts": contracts.len(), "behavior_verified": false}))
    }
}

fn validate(tx: &rusqlite::Transaction<'_>, generation: i64, contract: &Contract) -> Result<()> {
    if contract.module.is_empty()
        || contract.module.len() > 256
        || contract.interface.len() > MAX_CONTENT
        || contract.abi.len() > MAX_CONTENT
        || contract.unknowns.len() > MAX_CONTENT
    {
        return Err(failure("Contract content exceeds its bounded size"));
    }
    if contract.ready && (contract.interface.is_empty() || contract.abi.is_empty()) {
        return Err(failure(
            "A ready contract must include interface and ABI content",
        ));
    }
    modules::ensure_module(tx, generation, &contract.module)
}

fn digest(content: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(content.as_bytes());
    format!("{:x}", hash.finalize())
}
