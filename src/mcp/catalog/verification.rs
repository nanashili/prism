use super::tool;
use serde_json::{Value, json};

pub(super) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "verification_evidence",
            "Capture Prism-owned evidence or read an immutable snapshot. capture requires view and address and reuses saved pseudocode; it never starts decompilation. analysis_call replies also return prism_evidence receipts. latest_runtime reads the last captured debugger observation for the opened binary; agents cannot upload evidence or set its provenance. Old snapshots remain readable with current=false after analysis changes.",
            &["action"],
            json!({
                "action":{"type":"string","enum":["capture","get","latest_runtime"]},
                "view":{"type":"string","enum":["pseudocode","assembly","references"]},
                "address":{"type":"string","minLength":1,"maxLength":1024},
                "evidence":{"type":"string","minLength":64,"maxLength":64}
            }),
        ),
        tool(
            "verification_claim",
            "Record and verify substantive agent claims. propose/revise requires draft; verify/get/revise requires claim. list is paginated. Every draft requires captured evidence references, JSON pointers (empty selects the whole result), and equals or contains predicates. Status and confidence are computed by Prism and cannot be supplied. scope=predicate verifies only the declared facts; behavioral interpretations must use scope=interpretation and remain unresolved until semantic verification exists. A failed text search never disproves behavior. Cite the returned [claim:ID] in your answer. Revise reopened claims with fresh evidence before verifying again.",
            &["action"],
            json!({
                "action":{"type":"string","enum":["propose","verify","get","list","revise"]},
                "claim":{"type":"integer","minimum":1},
                "offset":{"type":"integer","minimum":0},
                "draft":{
                    "type":"object","required":["statement","scope","checks"],"additionalProperties":false,
                    "properties":{
                        "statement":{"type":"string","minLength":1,"maxLength":4096},
                        "scope":{"type":"string","enum":["predicate","interpretation"]},
                        "checks":{"type":"array","minItems":1,"maxItems":16,"items":{
                            "type":"object","required":["evidence","pointer","predicate"],"additionalProperties":false,
                            "properties":{
                                "evidence":{"type":"string","minLength":64,"maxLength":64},
                                "pointer":{"type":"string","maxLength":1024},
                                "predicate":{
                                    "type":"object","required":["operation"],"additionalProperties":false,
                                    "properties":{
                                        "operation":{"type":"string","enum":["equals","contains"]},
                                        "expected":{},
                                        "text":{"type":"string","minLength":1,"maxLength":4096}
                                    }
                                }
                            }
                        }}
                    }
                }
            }),
        ),
    ]
}
