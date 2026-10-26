use super::tool;
use serde_json::{Value, json};

pub(super) fn definitions() -> Vec<Value> {
    vec![
        super::decompilation::definition(),
        super::type_recovery::definition(),
        tool(
            "list_functions",
            "Read up to 300 indexed functions. Analyze the file first. Offset starts at zero.",
            &[],
            json!({"filter":{"type":"string","maxLength":1024},"offset":{"type":"integer","minimum":0}}),
        ),
        tool(
            "read_bytes",
            "Read up to 4096 bytes at a file offset, not a virtual address.",
            &["offset"],
            json!({"offset":{"type":"integer","minimum":0},"length":{"type":"integer","minimum":1,"maximum":4096}}),
        ),
        tool(
            "analysis_tools",
            "List tools exposed by the open file's analysis provider. Backend annotation, file-switch and direct procedure_pseudo_code tools are excluded for every provider; use Prism's file, annotation and batch tools instead.",
            &[],
            json!({}),
        ),
        tool(
            "analysis_call",
            "Call an analysis tool using the current window's worker. Use analysis_tools for names and schemas. Use decompile_batch for decompilation, inspection and explanation, regardless of saved reconstruction state. Use reconstruction_evidence only for explicitly requested reconstruction work. Direct procedure_pseudo_code, backend annotation and file-switch tools are blocked for every provider. Evidence may be incomplete.",
            &["name"],
            json!({"name":{"type":"string","minLength":1,"maxLength":256},"arguments":{"type":"object"}}),
        ),
    ]
}
