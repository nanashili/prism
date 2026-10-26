use super::State;
use crate::error::{Result, failure};
use crate::model::View;
use crate::storage::verification::{ClaimDraft, EvidenceSource, SourceKind};
use serde_json::{Value, json};

mod debug;

impl State {
    pub(super) fn sync_verification_context(&self) -> Result<String> {
        let context = self.decompilation_context()?;
        let identity = super::analysis_cache_identity::namespace(&context);
        self.store
            .verification_context(&context.project, &identity)?;
        Ok(context.project)
    }

    pub(super) fn verification_claim_tool(&self, args: &Value) -> Result<Value> {
        let project = self.sync_verification_context()?;
        match args["action"].as_str() {
            Some("propose") => {
                let draft: ClaimDraft = serde_json::from_value(args["draft"].clone())?;
                self.store.propose_claim(&project, &draft)
            }
            Some("revise") => {
                let draft: ClaimDraft = serde_json::from_value(args["draft"].clone())?;
                self.store.revise_claim(&project, claim_id(args)?, &draft)
            }
            Some("verify") => self.store.verify_claim(&project, claim_id(args)?),
            Some("get") => self.store.verification_claim(&project, claim_id(args)?),
            Some("list") => self
                .store
                .verification_claims(&project, args["offset"].as_u64().unwrap_or(0)),
            _ => Err(failure("Unsupported claim action")),
        }
    }

    pub(super) fn verification_evidence_tool(&mut self, args: &Value) -> Result<Value> {
        let project = self.sync_verification_context()?;
        match args["action"].as_str() {
            Some("get") => self.store.verification_evidence(
                &project,
                args["evidence"]
                    .as_str()
                    .ok_or("An evidence identifier is required")?,
            ),
            Some("latest_runtime") => self.store.latest_runtime_evidence(&project),
            Some("capture") => self.capture_view_evidence(&project, args),
            _ => Err(failure("Unsupported evidence action")),
        }
    }

    fn capture_view_evidence(&mut self, project: &str, args: &Value) -> Result<Value> {
        let address = args["address"]
            .as_str()
            .ok_or("A function address is required")?;
        let view = match args["view"].as_str() {
            Some("pseudocode") => View::Pseudocode,
            Some("assembly") => View::Assembly,
            Some("references") => View::References,
            _ => return Err(failure("Select pseudocode, assembly, or references")),
        };
        self.require_indexed_function(address)?;
        let canonical = crate::analysis::graph::address_value(address)
            .map_or_else(|| address.to_owned(), |value| format!("0x{value:x}"));
        let data = self.evidence_view_data(project, view, &canonical)?;
        let key = format!("view:{}:{canonical}", view.label());
        let receipt = self.store.capture_evidence(
            project,
            &EvidenceSource {
                key: &key,
                subject: &canonical,
                kind: SourceKind::StaticAnalysis,
            },
            &data,
        )?;
        Ok(json!({"prism_evidence":receipt,"data":data}))
    }

    fn evidence_view_data(&mut self, project: &str, view: View, address: &str) -> Result<Value> {
        let text = if view == View::Pseudocode {
            let context = self.decompilation_context()?;
            self.store.durable_pseudocode(project, &context.provider, &context.configuration, address)?
                .ok_or("Run a durable decompile_batch job for this function before capturing pseudocode evidence")?
        } else {
            self.engine_view(view, address)?
        };
        if view == View::References {
            Ok(serde_json::from_str(&text)?)
        } else {
            Ok(json!({"text":text}))
        }
    }

    pub(super) fn capture_analysis_reply(
        &self,
        name: &str,
        args: &Value,
        reply: Value,
    ) -> Result<Value> {
        if reply["isError"] == true || reply.get("error").is_some_and(|error| !error.is_null()) {
            return Ok(reply);
        }
        let project = self.sync_verification_context()?;
        let key = serde_json::to_string(&("analysis", name, args))?;
        let subject = args["procedure"]
            .as_str()
            .or_else(|| args["address"].as_str())
            .unwrap_or("");
        let canonical = crate::analysis::graph::address_value(subject)
            .map_or_else(|| subject.to_owned(), |value| format!("0x{value:x}"));
        let receipt = self.store.capture_evidence(
            &project,
            &EvidenceSource {
                key: &key,
                subject: &canonical,
                kind: SourceKind::StaticAnalysis,
            },
            &reply,
        )?;
        // Keep provider fields intact, including scalars, and put the receipt
        // outside their namespace so a provider cannot forge Prism provenance.
        Ok(json!({"result":reply,"prism_evidence":receipt}))
    }
}

fn claim_id(args: &Value) -> Result<i64> {
    let id = i64::try_from(
        args["claim"]
            .as_u64()
            .ok_or("A claim identifier is required")?,
    )?;
    if id <= 0 {
        return Err(failure("Claim identifiers must be positive"));
    }
    Ok(id)
}
