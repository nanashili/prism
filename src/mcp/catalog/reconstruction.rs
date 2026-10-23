use super::tool;
use serde_json::{Value, json};

pub(super) fn definitions() -> Vec<Value> {
    vec![
        definition(
            "reconstruction_start",
            "Create or resume a persistent source reconstruction generation only when the user explicitly requests reconstruction or a source port. A decompilation request uses decompile_batch, even if a generation exists. Any target language is accepted. new_generation freezes the previous generation and reuses matching source and evidence. Root must exist; it is bound to this input binary.",
            &["root", "language"],
            json!({
                "language":{"type":"string","minLength":1,"maxLength":80},
                "instructions":{"type":"string","maxLength":32768},
                "model":{"type":"string","minLength":1,"maxLength":256},
                "prompt_version":{"type":"string","minLength":1,"maxLength":256},
                "adapter_version":{"type":"string","minLength":1,"maxLength":256},
                "validation_version":{"type":"string","minLength":1,"maxLength":256},
                "batch_size":{"type":"integer","minimum":1,"maximum":64},
                "workers":{"type":"integer","minimum":1,"maximum":8},
                "new_generation":{"type":"boolean"}
            }),
        ),
        definition(
            "reconstruction_status",
            "Read counts, generation history, reuse and the latest revision-specific build receipt. Generated source is an agent claim; behavioral equivalence is unverified.",
            &[],
            json!({}),
        ),
        definition(
            "reconstruction_claim",
            "Claim a bounded module batch for explicitly requested source reconstruction, never for a plain decompile, inspect or explain request. A saved generation alone does not authorize this workflow. Leases last 15 minutes and are renewable. First complete __project__ with the project structure and shared interfaces. A repeated worker name returns its active batch. No eligible work is not a completion claim.",
            &["worker"],
            json!({
                "worker":{"type":"string","minLength":1,"maxLength":120}
            }),
        ),
        definition(
            "reconstruction_evidence",
            "Read cached pseudocode and local analysis for four functions in an active source reconstruction batch. Requires explicitly requested reconstruction; use decompile_batch for ordinary decompilation even when a generation exists. Page by offset before submitting generated source. Does not build a whole-program callers index. Errors must be resolved or recorded as blockers.",
            &["lease"],
            json!({
                "lease":{"type":"string","minLength":32,"maxLength":32},
                "offset":{"type":"integer","minimum":0,"maximum":64}
            }),
        ),
        definition(
            "reconstruction_submit",
            "Atomically save UTF-8 source and every unit result in an explicitly requested source reconstruction lease. Do not submit reconstruction results for a plain decompilation request. Up to 64 files, 1 MiB per file, 4 MiB total. Use current previous_sha256, or empty string for a new file. Conflicts require a fresh read and merge. Identical retries are idempotent. Generated functions require evidence and paths; blocked functions require a concrete reason.",
            &["submission"],
            submission(),
        ),
        definition(
            "reconstruction_units",
            "Page through 64 inventory units, states and blockers. Includes __project__ for project structure, resources and integration.",
            &[],
            json!({
                "state":{"type":"string","enum":["","pending","leased","generated","reused","blocked"]},
                "offset":{"type":"integer","minimum":0,"maximum":1000000}
            }),
        ),
        definition(
            "reconstruction_files",
            "List 64 generation source paths, SHA-256 identities and byte sizes.",
            &[],
            json!({"offset":{"type":"integer","minimum":0,"maximum":1000000}}),
        ),
        definition(
            "reconstruction_read_file",
            "Read a source file and its hash before revising or merging it. Paths are relative to the generation, independent of export snapshots.",
            &["path"],
            json!({"path":{"type":"string","minLength":1,"maxLength":1024}}),
        ),
        definition(
            "reconstruction_control",
            "Pause/resume a generation, retry blocked work, reopen one address for refinement, or renew/release a batch lease. Pausing releases all active batches. Earlier frozen generations cannot be changed.",
            &["action"],
            json!({
                "action":{"type":"string","enum":["pause","resume","retry_blocked","reopen","renew","release"]},
                "lease":{"type":"string","minLength":32,"maxLength":32},
                "address":{"type":"string","minLength":1,"maxLength":1024}
            }),
        ),
        definition(
            "reconstruction_build",
            "Export a fresh revision-specific source snapshot and run its build, with diagnostics recorded. Set export_only to skip execution. Defaults support Rust, C/C++, C#, Swift, Go, TypeScript, Maven Java, Zig and Python syntax compilation. Other languages/build systems need command. Inspect generated build scripts first. A successful build does not verify behavior.",
            &[],
            json!({
                "export_only":{"type":"boolean"},
                "command":{"type":"object","required":["program","args"],"additionalProperties":false,
                    "properties":{"program":{"type":"string"},"args":{"type":"array","items":{"type":"string"}}}}
            }),
        ),
        definition(
            "reconstruction_report",
            "Export a streamed JSON report of this generation's inventory, unresolved work, reuse and latest build receipt. Does not claim equivalence.",
            &[],
            json!({}),
        ),
    ]
}

pub(super) fn definition(
    name: &str,
    description: &str,
    required: &[&str],
    mut properties: Value,
) -> Value {
    properties["root"] = json!({"type":"string","minLength":1,"maxLength":4096,
        "description":"Absolute reconstruction root. Omit to use this project's last root."});
    if name != "reconstruction_start" {
        properties["generation"] = json!({"type":"integer","minimum":1,
            "description":"Generation ID; defaults to the latest."});
    }
    tool(name, description, required, properties)
}

fn submission() -> Value {
    json!({"submission":{"type":"object","required":["lease","files","results"],"additionalProperties":false,
    "properties":{
        "lease":{"type":"string"},
        "files":{"type":"array","maxItems":64,"items":{"type":"object",
            "required":["path","content","previous_sha256"],"additionalProperties":false,
            "properties":{"path":{"type":"string"},"content":{"type":"string"},"previous_sha256":{"type":"string"}}}},
        "results":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"object",
            "required":["address","state","reason","paths"],"additionalProperties":false,
            "properties":{"address":{"type":"string"},"state":{"type":"string","enum":["generated","blocked"]},
                "reason":{"type":"string"},"paths":{"type":"array","items":{"type":"string"}}}}}
    }}})
}
