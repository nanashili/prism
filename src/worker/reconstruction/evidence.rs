use super::State;
#[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
use crate::analysis::DecompilationItem;
use crate::error::{Result, failure};
use crate::reconstruction::Database;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

pub(super) struct EvidencePage {
    pub(super) items: Vec<Value>,
    pub(super) fresh: Vec<(String, usize)>,
    bytes: usize,
}

impl EvidencePage {
    fn new(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            fresh: Vec::with_capacity(capacity),
            bytes: 0,
        }
    }

    fn push(&mut self, address: String, cached: bool, item: Value) -> Result<bool> {
        let Some(evidence) = item.get("evidence") else {
            self.items.push(item);
            return Ok(true);
        };
        let size = serde_json::to_vec(evidence)?.len();
        if size > 512 * 1024 {
            self.items.push(json!({
                "address": address,
                "error": "Evidence exceeds the function budget. Inspect assembly or record a concrete blocker."
            }));
            return Ok(true);
        }
        if self.bytes + size > 1008 * 1024 {
            return Ok(false);
        }

        self.bytes += size;
        let index = self.items.len();
        self.items.push(item);
        if !cached {
            self.fresh.push((address, index));
        }
        Ok(true)
    }
}

impl State {
    pub(super) fn collect_evidence(
        &mut self,
        database: &Database,
        units: Vec<(String, Option<String>)>,
    ) -> Result<EvidencePage> {
        let addresses = units
            .iter()
            .filter(|(_, cached)| cached.is_none())
            .map(|(address, _)| address.clone())
            .collect();
        let mut fresh = self.reconstruction_batch_evidence(addresses)?;
        let mut page = EvidencePage::new(units.len());

        for (address, cached) in units {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            let item = match cached.as_deref() {
                Some(digest) => cached_item(database, &address, digest),
                None => fresh
                    .remove(&address)
                    .ok_or("The reconstruction batch omitted a saved address")?,
            };
            if !page.push(address, cached.is_some(), item)? {
                break;
            }
        }
        Ok(page)
    }

    fn reconstruction_batch_evidence(
        &mut self,
        addresses: Vec<String>,
    ) -> Result<HashMap<String, Value>> {
        if addresses.is_empty() {
            return Ok(HashMap::new());
        }
        if addresses.len() > 4 {
            return Err(failure(
                "Reconstruction evidence requires a page of at most four functions",
            ));
        }

        #[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
        if self.settings.analysis_backend().retains_prepared_session() {
            let items = self.saved_decompilation_batch(addresses)?;
            return Ok(items.into_iter().map(batch_item).collect());
        }

        Err(failure(
            "Fresh reconstruction evidence requires a prepared Ghidra REA batch provider. Cached evidence remains available with this provider.",
        ))
    }
}

#[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
fn batch_item(item: DecompilationItem) -> (String, Value) {
    let mut value = json!({"address": item.address, "cached": false});
    match item.result {
        Ok(code) => {
            value["evidence"] = json!({"pseudocode": code, "behavior_verified": false});
        }
        Err(error) => {
            value["error"] = error.into();
            if let Some(details) = item.error_details {
                value["error_details"] = details;
            }
        }
    }
    (item.address, value)
}

fn cached_item(database: &Database, address: &str, digest: &str) -> Value {
    match database.cached_evidence(digest) {
        Ok(evidence) => json!({"address": address, "cached": true, "evidence": evidence}),
        Err(error) => json!({"address": address, "error": error.to_string()}),
    }
}
