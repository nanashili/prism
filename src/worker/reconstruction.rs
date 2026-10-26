use super::State;
use crate::error::{Result, failure};
use crate::events::Event;
use crate::reconstruction::DatabaseOpen;
use crate::reconstruction::{self, Database, EvidenceItem};
use serde_json::{Value, json};

mod evidence;

impl State {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn reconstruct(&mut self, name: &str, args: &Value) -> Result<Value> {
        let project = self
            .project
            .as_ref()
            .ok_or("Open a project first")?
            .target
            .sha256
            .clone();
        if name == "reconstruction_start" && !self.store.has_analysis(&project)? {
            self.index()?;
        }
        let context = self.decompilation_context()?;
        let analyzer = crate::worker::analysis_cache_identity::namespace(&context);
        let result = match name {
            "reconstruction_evidence" => self.reconstruction_evidence(&project, &analyzer, args),
            "reconstruction_build" => {
                reconstruction::build_generation(&project, args, &self.cancelled)
            }
            _ => reconstruction::call(&self.store, &project, &analyzer, name, args),
        }?;
        if matches!(
            name,
            "reconstruction_units" | "reconstruction_files" | "reconstruction_read_file"
        ) {
            return Ok(result);
        }
        let status = if matches!(
            name,
            "reconstruction_start" | "reconstruction_control" | "reconstruction_status"
        ) {
            result.clone()
        } else {
            let root = reconstruction::root(&project, args)?;
            let database = Database::open(&root, DatabaseOpen::Existing)?;
            let generation = database.generation(args["generation"].as_i64())?;
            database.status(generation)?
        };
        self.sink.send(Event::Reconstruction {
            project,
            status: Box::new(status),
            start_agent: matches!(name, "reconstruction_start" | "reconstruction_control")
                && args["start_agent"] == true,
        })?;
        Ok(result)
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    fn reconstruction_evidence(
        &mut self,
        project: &str,
        analyzer: &str,
        args: &Value,
    ) -> Result<Value> {
        let root = reconstruction::root(project, args)?;
        let mut database = Database::open(&root, DatabaseOpen::Existing)?;
        database.verify_project(project)?;
        let generation = database.generation(args["generation"].as_i64())?;
        let recorded: String = database.db.query_row(
            "SELECT analyzer FROM generations WHERE id=?1",
            [generation],
            |row| row.get(0),
        )?;
        if analyzer != recorded {
            return Err(failure(
                "The analysis provider changed. Start a new generation to refresh its evidence",
            ));
        }
        let lease = args["lease"].as_str().ok_or("Supply a lease token")?;
        let offset = args["offset"].as_u64().unwrap_or_default();
        let units = database.evidence_units(generation, lease, offset)?;
        let page = self.collect_evidence(&database, units)?;
        if !page.fresh.is_empty() {
            let fresh = page
                .fresh
                .iter()
                .map(|(address, index)| {
                    Ok(EvidenceItem {
                        address,
                        evidence: page.items[*index]
                            .get("evidence")
                            .ok_or("Fresh evidence response is malformed")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            database.save_evidence_batch(generation, lease, &fresh)?;
        }
        let next = offset + u64::try_from(page.items.len())?;
        let more = !database.evidence_units(generation, lease, next)?.is_empty();
        let addresses: Vec<&str> = page
            .items
            .iter()
            .filter_map(|item| item["address"].as_str())
            .collect();
        let types = if addresses.is_empty() {
            None
        } else {
            self.store
                .recovered_type_context(project, analyzer, &addresses)?
        };
        Ok(
            json!({"items":page.items,"offset":offset,"page_size":4,"next_offset":more.then_some(next),"type_recovery":types}),
        )
    }
}
