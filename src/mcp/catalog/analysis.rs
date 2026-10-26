use super::tool;
use serde_json::{Value, json};

pub(super) fn definitions() -> Vec<Value> {
    vec![
        super::decompilation::definition(),
        tool(
            "index_function_search",
            "Index a page of up to 300 saved native Rust Ghidra functions and their cached/durable evidence; follow next_offset until null. Builds normalized decompiler/CFG structural signatures in Rust. Does not run analysis or decompilation. Refresh after changing evidence or notes. reset=true clears derived search indexes for this binary across configurations. Requires the native Rust Ghidra backend.",
            &[],
            json!({"offset":{"type":"integer","minimum":0},"reset":{"type":"boolean"}}),
        ),
        tool(
            "semantic_search",
            "Find indexed native functions by behavior, e.g. 'parse a binary header' or 'implement an object pool'. Returns ranked evidence, excerpts and reasons. Uses term/synonym retrieval, behavior heuristics and native Rust decompiler/CFG structural similarity, not BSim or trained embeddings. Run index_function_search first. similar_to optionally selects a structural exemplar address. Scores are leads, not verified behavior.",
            &["query"],
            json!({"query":{"type":"string","minLength":1,"maxLength":1024},"limit":{"type":"integer","minimum":1,"maximum":100},"similar_to":{"type":"string","minLength":1,"maxLength":64}}),
        ),
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
