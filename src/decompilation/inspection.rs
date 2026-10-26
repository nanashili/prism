use super::{Context, ExportRunner};
use crate::error::{Result, failure};
use crate::storage::{DecompilationJob, Store};
use serde_json::{Value, json};

impl ExportRunner {
    pub(crate) fn parse_job_id(args: &Value) -> Result<&str> {
        let id = args["job"].as_str().ok_or("A durable job id is required")?;
        if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(failure("The durable job id must be a SHA-256 identifier"));
        }
        Ok(id)
    }

    pub(crate) fn status(store: &Store, id: &str, context: &Context) -> Result<Value> {
        let job = Self::load(store, id)?;
        verify_project(&job, context)?;
        let mut result = summary(&job);
        result["configuration_matches"] = json!(context.matches(&job));
        Ok(result)
    }

    pub(crate) fn results(
        store: &Store,
        id: &str,
        context: &Context,
        args: &Value,
    ) -> Result<Value> {
        let job = Self::load(store, id)?;
        verify_project(&job, context)?;
        let offset = page_offset(args)?;
        let limit = page_limit(args)?;
        let include_data = args["include_data"].as_bool().unwrap_or(false);
        let (rows, more) = store.decompilation_results_page(id, offset, limit, include_data)?;
        let next = offset + rows.len();
        let saved_context = Context {
            project: job.project,
            provider: job.provider,
            configuration: job.options,
        };
        let analyzer = crate::worker::analysis_cache_identity::namespace(&saved_context);
        let addresses: Vec<&str> = rows.iter().map(|row| row.address.as_str()).collect();
        let types = if addresses.is_empty() {
            None
        } else {
            store.recovered_type_context(&context.project, &analyzer, &addresses)?
        };
        Ok(json!({"job": id, "offset": offset, "results": rows,
            "next_offset": more.then_some(next), "type_recovery": types}))
    }

    pub(crate) fn list(store: &Store, context: &Context, args: &Value) -> Result<Value> {
        let offset = page_offset(args)?;
        let limit = page_limit(args)?.min(32);
        let rows = store.list_decompilation_jobs(&context.project, offset, limit + 1)?;
        let more = rows.len() > limit;
        let jobs = rows.iter().take(limit).map(summary).collect::<Vec<_>>();
        Ok(json!({"jobs": jobs, "next_offset": more.then_some(offset + limit)}))
    }

    pub(crate) fn control_saved(
        store: &Store,
        id: &str,
        context: &Context,
        action: &str,
    ) -> Result<Value> {
        let _ownership = super::ownership::Ownership::acquire(&context.project)?;
        control(store, id, context, action)
    }

    pub(crate) fn control(
        &mut self,
        store: &Store,
        context: &Context,
        action: &str,
    ) -> Result<Value> {
        control(store, &self.job.id, context, action)
    }
}

fn control(store: &Store, id: &str, context: &Context, action: &str) -> Result<Value> {
    let job = ExportRunner::load(store, id)?;
    verify_project(&job, context)?;
    match action {
        "cancel" => store.update_decompilation_state(id, "cancelled", None)?,
        "pause" => store.update_decompilation_state(id, "paused", None)?,
        "retry_failed" => {
            context.verify(&job)?;
            store.retry_decompilation_failures(id)?;
        }
        _ => return Err(failure("Unknown durable job control")),
    }
    ExportRunner::status(store, id, context)
}

pub(super) fn summary(job: &DecompilationJob) -> Value {
    json!({"id": job.id, "project": job.project, "provider": job.provider,
        "state": job.state, "total": job.total, "completed": job.completed,
        "failed": job.failed, "next_offset": job.next_offset, "error": job.error,
        "artifact_bytes": job.artifact_bytes,
        "selection": job.selection, "configuration": job.options,
        "behavior_verified": false})
}

fn verify_project(job: &DecompilationJob, context: &Context) -> Result<()> {
    if job.project != context.project {
        return Err(failure("The durable job belongs to a different project"));
    }
    Ok(())
}

fn page_offset(args: &Value) -> Result<usize> {
    usize::try_from(args["offset"].as_u64().unwrap_or_default()).map_err(Into::into)
}

fn page_limit(args: &Value) -> Result<usize> {
    let limit = args["limit"].as_u64().unwrap_or(32);
    if !(1..=100).contains(&limit) {
        return Err(failure("Use a page limit of 1–100"));
    }
    Ok(usize::try_from(limit)?)
}
