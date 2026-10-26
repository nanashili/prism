use super::tool;
use serde_json::{Value, json};

pub(super) fn definitions() -> Vec<Value> {
    vec![
        super::analysis_repair::definition(),
        super::type_recovery::definition(),
        super::decompilation::definition(),
        tool(
            "compare_versions",
            "Compare the open destination binary with a baseline app, binary or .prism project. Rust-native matching uses exact bytes, normalized instructions, structural signatures and matched callees; Rust REA listing spans are distinguished from recovered CFG/body evidence. Opens providers sequentially, pauses decompilation and returns a summary plus the first 100 matches. No annotation transfer occurs. Review candidates in Diff or use version_matches for other pages.",
            &["baseline"],
            json!({"baseline":{"type":"string","minLength":1,"maxLength":4096}}),
        ),
        tool(
            "version_matches",
            "Read the last version comparison in pages of 100. Ambiguous results return one compact group at offset, with separate source_offset and target_offset pages of 100. Confirmed baseline annotations accompany candidate pages. Unmatched functions are possible additions/removals; extraction failures are unknown. Duplicate groups and heuristic matches need review.",
            &[],
            json!({"kind":{"type":"string","enum":["matches","ambiguous","source_only","target_only","source_unknown","target_unknown"]},"offset":{"type":"integer","minimum":0},"source_offset":{"type":"integer","minimum":0},"target_offset":{"type":"integer","minimum":0}}),
        ),
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
        super::execution::definition(),
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
            "Discover tools exposed by the open file's analysis provider. The default summary returns names, full guidance and required arguments without nested schemas. Before calling an unfamiliar tool, request detail=full with its exact name in names; detail=full with no names preserves the complete allowed provider catalog. Backend annotation, file-switch and direct procedure_pseudo_code tools are excluded for every provider; use Prism's file, annotation and batch tools instead. Names accepts at most 32 exact tool names.",
            &[],
            json!({
                "detail":{"type":"string","enum":["summary","full"]},
                "names":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"string","minLength":1,"maxLength":256}}
            }),
        ),
        tool(
            "analysis_call",
            "Call an analysis tool using the current window's worker. Use analysis_tools for names and required arguments, then request detail=full with names before constructing unfamiliar nested arguments. Successful replies contain result (the provider reply) and prism_evidence (Prism's immutable snapshot receipt). Verification JSON pointers select inside the provider reply. Use decompile_batch for decompilation, inspection and explanation, regardless of saved reconstruction state. Use reconstruction_evidence only for explicitly requested reconstruction work. Direct procedure_pseudo_code, backend annotation and file-switch tools are blocked for every provider. Evidence may be incomplete.",
            &["name"],
            json!({"name":{"type":"string","minLength":1,"maxLength":256},"arguments":{"type":"object"}}),
        ),
    ]
}
