use super::Rea;
use super::batch_parse;
use super::batch_state::{
    Counts, DecompilationBatch, DecompilationBatchPoll, PendingCall, RUST_BATCH_SIZE, Request,
    Stage,
};
use super::reply::{self, ToolReply};
use crate::error::{Result, failure};
use crate::settings::AnalysisBackend;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

impl Rea {
    pub fn begin_decompilation_batch(
        &mut self,
        addresses: Vec<String>,
    ) -> Result<DecompilationBatch> {
        self.ensure_available()?;
        if !self.backend.retains_prepared_session() || self.target.is_none() {
            return Err(failure(
                "Open the project with the selected Ghidra REA provider before starting this batch",
            ));
        }
        validate_addresses(&addresses)?;
        let (call, request) = if self.backend == AnalysisBackend::RustGhidraRea {
            self.begin_rust_page(&addresses, 0)?
        } else {
            let call = self.begin_tool(
                "batch_decompile",
                json!({"addresses":addresses,"detail":"summary"}),
            )?;
            (call, Request::Summary)
        };
        let token = call.id;
        self.active_batch = Some(token);
        Ok(DecompilationBatch {
            session: self.session.clone(),
            token,
            addresses,
            evidence: None,
            offset: 0,
            page_limit: 64,
            expected: Counts::default(),
            observed: Counts::default(),
            stage: Stage::Waiting(call, request),
        })
    }

    pub fn poll_decompilation_batch(
        &mut self,
        batch: &mut DecompilationBatch,
    ) -> Result<DecompilationBatchPoll> {
        self.verify_batch(batch)?;
        let result = self.poll_batch(batch);
        if result.is_err() {
            if !matches!(batch.stage, Stage::Failed) {
                self.healthy = false;
            }
            self.active_batch = None;
        }
        result
    }

    pub fn abandon_decompilation_batch(&mut self, batch: DecompilationBatch) -> Result<()> {
        if !Arc::ptr_eq(&self.session, &batch.session) {
            return Err(failure(
                "The decompilation batch belongs to a previous Ghidra session",
            ));
        }
        if self.active_batch == Some(batch.token) {
            self.active_batch = None;
            if let Stage::Waiting(call, _) = batch.stage {
                self.abandoned = Some(call);
            }
        }
        Ok(())
    }

    fn verify_batch(&self, batch: &DecompilationBatch) -> Result<()> {
        if !self.healthy
            || !Arc::ptr_eq(&self.session, &batch.session)
            || (self.active_batch != Some(batch.token) && !matches!(batch.stage, Stage::Complete))
        {
            return Err(failure(
                "The prepared Ghidra batch is unavailable. Reopen the project before resuming.",
            ));
        }
        Ok(())
    }

    fn poll_batch(&mut self, batch: &mut DecompilationBatch) -> Result<DecompilationBatchPoll> {
        let stage = std::mem::replace(&mut batch.stage, Stage::Complete);
        match stage {
            Stage::Failed => Err(failure("This decompilation batch has already failed")),
            Stage::Complete => {
                self.active_batch = None;
                Ok(DecompilationBatchPoll::Complete)
            }
            Stage::Ready => {
                let (call, request) = self.begin_next_page(batch)?;
                batch.stage = Stage::Waiting(call, request);
                Ok(DecompilationBatchPoll::Pending)
            }
            Stage::Waiting(call, request) => {
                let Some(response) = self.poll_call(&call)? else {
                    batch.stage = Stage::Waiting(call, request);
                    return Ok(DecompilationBatchPoll::Pending);
                };
                let reply = self.parse_tool_reply(response)?;
                self.accept_reply(batch, request, reply)
            }
        }
    }

    fn begin_next_page(&mut self, batch: &DecompilationBatch) -> Result<(PendingCall, Request)> {
        if self.backend == AnalysisBackend::RustGhidraRea {
            return self.begin_rust_page(&batch.addresses, batch.offset);
        }

        let evidence = batch
            .evidence
            .as_deref()
            .ok_or("Missing retained batch evidence")?;
        let call = self.begin_tool(
            "inspect_analysis_view",
            json!({
                "source":{"kind":"retained-evidence","evidence_id":evidence},
                "view":{"kind":"page","collection":"decompilations",
                    "offset":batch.offset,"limit":batch.page_limit}
            }),
        )?;
        Ok((call, Request::Page))
    }

    fn begin_rust_page(
        &mut self,
        addresses: &[String],
        offset: usize,
    ) -> Result<(PendingCall, Request)> {
        let end = addresses.len().min(offset + RUST_BATCH_SIZE);
        let call = self.begin_tool(
            "batch_decompile",
            json!({"addresses":&addresses[offset..end]}),
        )?;
        Ok((call, Request::RustPage { end }))
    }

    fn accept_reply(
        &mut self,
        batch: &mut DecompilationBatch,
        request: Request,
        reply: ToolReply,
    ) -> Result<DecompilationBatchPoll> {
        match (request, reply) {
            (Request::Summary, ToolReply::Failure(error)) => {
                let Some(evidence) = error.retained_evidence().map(str::to_owned) else {
                    batch.stage = Stage::Failed;
                    return Err(error.into_error());
                };
                let call = self.begin_tool(
                    "inspect_analysis_view",
                    json!({
                        "source":{"kind":"retained-evidence","evidence_id":evidence},
                        "view":{"kind":"summary"}
                    }),
                )?;
                batch.evidence = Some(evidence);
                batch.stage = Stage::Waiting(call, Request::RetainedSummary);
                Ok(DecompilationBatchPoll::Pending)
            }
            (Request::Page, ToolReply::Failure(error))
                if error.resource_constraint() && batch.page_limit > 1 =>
            {
                batch.page_limit /= 2;
                batch.stage = Stage::Ready;
                Ok(DecompilationBatchPoll::Pending)
            }
            (_, ToolReply::Failure(error)) => {
                batch.stage = Stage::Failed;
                Err(error.into_error())
            }
            (request, ToolReply::Success(value)) => {
                let value = reply::normalized(value);
                let items = match request {
                    Request::Summary | Request::RetainedSummary => {
                        batch_parse::summary(&value, batch)?;
                        batch.stage = Stage::Ready;
                        return Ok(DecompilationBatchPoll::Pending);
                    }
                    Request::Page => batch_parse::page(value, batch)?,
                    Request::RustPage { end } => batch_parse::rust_page(value, batch, end)?,
                };
                batch.stage = if batch.offset == batch.addresses.len() {
                    Stage::Complete
                } else {
                    Stage::Ready
                };
                Ok(DecompilationBatchPoll::Page(items))
            }
        }
    }
}

fn validate_addresses(addresses: &[String]) -> Result<()> {
    if addresses.is_empty() || addresses.len() > 256 {
        return Err(failure(
            "A Ghidra provider batch requires between 1 and 256 saved addresses",
        ));
    }
    let mut unique = HashSet::with_capacity(addresses.len());
    for address in addresses {
        let value = crate::analysis::graph::address_value(address)
            .filter(|_| address.len() <= 128)
            .ok_or("Invalid saved function address")?;
        if !unique.insert(value) {
            return Err(failure(
                "A Ghidra provider batch contains duplicate saved addresses",
            ));
        }
    }
    Ok(())
}
