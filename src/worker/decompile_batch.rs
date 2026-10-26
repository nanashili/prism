use super::State;
use crate::error::{Result, failure};
use crate::model::View;
use serde_json::{Value, json};
use std::sync::atomic::Ordering;

const MAX_PSEUDOCODE_BYTES: usize = 512 * 1024;
const MAX_BATCH_RESPONSE_BYTES: usize = 1008 * 1024;

impl State {
    pub(super) fn single_address_decompile_batch(&mut self, address: &str) -> Result<String> {
        let reply = self.decompile_batch(&json!({"procedures":[address]}))?;
        let items = reply["items"]
            .as_array()
            .ok_or("The one-address decompilation batch returned no result list")?;
        let [item] = items.as_slice() else {
            return Err(failure(
                "The one-address decompilation batch changed its saved address count",
            ));
        };
        let returned_address = item["address"]
            .as_str()
            .ok_or("The one-address decompilation batch omitted its address")?;
        if returned_address != address {
            return Err(failure(
                "The one-address decompilation batch returned a different function",
            ));
        }
        item["pseudocode"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| {
                failure(
                    item["error"]
                        .as_str()
                        .unwrap_or("The one-address decompilation batch failed"),
                )
            })
    }

    pub(super) fn decompile_batch(&mut self, args: &Value) -> Result<Value> {
        if args.get("action").is_some() {
            return self.durable_decompile_batch(args);
        }
        #[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
        if self.settings.analysis_backend().retains_prepared_session() {
            return Err(failure(
                "The prepared Ghidra provider uses durable batches. Supply action=start with addresses or filter, then read status and results using the returned job ID.",
            ));
        }
        let procedures = args["procedures"]
            .as_array()
            .ok_or("Supply an array of function addresses")?;
        if procedures.is_empty() || procedures.len() > 64 {
            return Err(failure("Supply 1–64 addresses per decompilation batch"));
        }
        for procedure in procedures {
            if procedure.as_str().is_none_or(|address| {
                address.len() > 1024 || crate::analysis::graph::address_value(address).is_none()
            }) {
                return Err(failure("Every procedure must be a valid function address"));
            }
        }
        let offset = usize::try_from(args["offset"].as_u64().unwrap_or_default())?;
        if offset > procedures.len() {
            return Err(failure("The batch offset exceeds its address count"));
        }

        let mut items = Vec::with_capacity(4);
        let mut bytes = 0;
        let mut next = offset;
        for procedure in procedures.iter().skip(offset).take(4) {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            let address = procedure.as_str().ok_or("Invalid procedure address")?;
            let result = self
                .require_indexed_function(address)
                .and_then(|()| self.read_view_strict(View::Pseudocode, address, 0));
            let mut item = match result {
                Ok(code) if code.len() <= MAX_PSEUDOCODE_BYTES => {
                    json!({"address":address,"pseudocode":code})
                }
                Ok(_) => {
                    json!({"address":address,"error":"Pseudocode exceeds the 512 KiB function limit. Inspect assembly or record a blocker."})
                }
                Err(error) => json!({"address":address,"error":error.to_string()}),
            };
            let mut size = serde_json::to_vec(&item)?.len();
            if size > MAX_PSEUDOCODE_BYTES {
                item = json!({"address":address,"error":"Serialized pseudocode exceeds the 512 KiB function limit. Inspect assembly or record a blocker."});
                size = serde_json::to_vec(&item)?.len();
            }
            if let Some(pseudocode) = item["pseudocode"].as_str() {
                self.cache_accepted_batch_pseudocode(address, pseudocode)?;
            }
            if bytes + size > MAX_BATCH_RESPONSE_BYTES {
                break;
            }

            bytes += size;
            items.push(item);
            next += 1;
        }
        let context = self.decompilation_context()?;
        let analyzer = super::analysis_cache_identity::namespace(&context);
        let addresses: Vec<&str> = items
            .iter()
            .filter_map(|item| item["address"].as_str())
            .collect();
        let types = if addresses.is_empty() {
            None
        } else {
            self.store
                .recovered_type_context(&context.project, &analyzer, &addresses)?
        };
        Ok(
            json!({"items":items,"offset":offset,"page_size":4,"type_recovery":types,
            "next_offset":(next < procedures.len()).then_some(next),
            "total":procedures.len(),"behavior_verified":false}),
        )
    }
}
