use crate::decompilation::ExportRunner;
use crate::error::Result;
use crate::worker::RuntimeSnapshot;
use serde_json::{Value, json};

pub(super) fn durable_batch(
    tool: &str,
    args: &Value,
    runtime: &RuntimeSnapshot,
) -> Result<Option<Value>> {
    if tool != "decompile_batch" {
        return Ok(None);
    }

    let Some(action) = args["action"].as_str() else {
        return Ok(None);
    };
    if !matches!(action, "list" | "status" | "results") {
        return Ok(None);
    }

    let (context, active_job) = runtime.read()?;
    let database = crate::settings::support_dir().join("prism.db");
    let store = crate::storage::Store::open_readonly(&database)?;
    let result = match action {
        "list" => ExportRunner::list(&store, &context, args)?,
        "status" => {
            let id = ExportRunner::parse_job_id(args)?;
            let mut result = ExportRunner::status(&store, id, &context)?;
            result["active_in_window"] = json!(active_job.as_deref() == Some(id));
            result
        }
        "results" => {
            let id = ExportRunner::parse_job_id(args)?;
            ExportRunner::results(&store, id, &context, args)?
        }
        _ => unreachable!(),
    };
    Ok(Some(result))
}
