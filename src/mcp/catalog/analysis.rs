use super::tool;
use serde_json::{Value, json};

pub(super) fn definitions() -> Vec<Value> {
    vec![
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
