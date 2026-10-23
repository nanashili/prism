use super::tool;
use serde_json::{Value, json};

pub(super) fn definition() -> Value {
    tool(
        "decompile_batch",
        &format!(
            concat!(
                "Always use this batch command for decompilation. Select only addresses already in the saved analysis using list_functions. ",
                "Unknown addresses fail; this command never discovers functions. Do not run analyze or discover_functions to expand a decompilation request. ",
                "Prefer action=start with addresses for a focused selection, or filter/all inventory for a module or project. This freezes the saved selection and runs a durable background job. ",
                "Ghidra (Java through REA) and Ghidra (Rust through REA) require this durable path, including one-item selections; they reuse a session prepared by explicitly opening the project. Other engines also support legacy procedures (1–64 addresses) with next_offset. ",
                "An explicit durable selection accepts 1 to {durable_address_limit} indexed addresses within the 8 MiB request limit. ",
                "Prefer filter or omit addresses/filter for a whole project or larger selection. Keep the returned job id. ",
                "Use list after restart, then status, resume, results, pause or cancel. ",
                "retry_failed preserves successful artifacts and pauses at the first failure; use it only after a diagnosed provider, configuration or input cause has changed, then resume to process them. ",
                "Results are paged and include pseudocode only with include_data=true. Saved output survives app restart; reopening does not automatically restart a job. ",
                "Configuration changes require matching settings or a new job. Each function is bounded to 512 KiB; each job has an 8 GiB compressed artifact quota. ",
                "This workflow never claims reconstruction work or generates replacement source."
            ),
            durable_address_limit = super::super::MAX_DURABLE_ADDRESSES,
        ),
        &[],
        json!({
            "action":{"type":"string","enum":["start","list","status","resume","results","pause","cancel","retry_failed"]},
            "procedures":{"type":"array","minItems":1,"maxItems":64,
                "items":{"type":"string","minLength":1,"maxLength":1024}},
            "addresses":{"type":"array","minItems":1,"maxItems":super::super::MAX_DURABLE_ADDRESSES,
                "items":{"type":"string","minLength":1,"maxLength":1024}},
            "filter":{"type":"string","maxLength":1024},
            "job":{"type":"string","minLength":64,"maxLength":64},
            "offset":{"type":"integer","minimum":0,"maximum":1000000},
            "limit":{"type":"integer","minimum":1,"maximum":100},
            "include_data":{"type":"boolean"}
        }),
    )
}
