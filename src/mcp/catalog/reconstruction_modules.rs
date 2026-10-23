use super::reconstruction::definition;
use serde_json::{Value, json};

pub(super) fn definitions() -> Vec<Value> {
    vec![
        definition(
            "reconstruction_modules",
            "Page through module contracts, readiness, dependencies and computed cycle groups. Follow next_offset. Contracts are proposed reconstruction interfaces, not proof of recovered behavior.",
            &[],
            json!({"offset":{"type":"integer","minimum":0,"maximum":1000000}}),
        ),
        definition(
            "reconstruction_dependencies",
            "Replace this generation's dependency graph using observed module relationships. Supply module -> depends_on edges; an empty array removes all edges. Limited to 4096 graph nodes and 16384 edges. Prism computes strongly connected components. Release active leases first. Changed edges invalidate affected source acceptance and consumers while preserving files for merge.",
            &["dependencies"],
            json!({"dependencies":{"type":"array","maxItems":16384,"items":{
                "type":"object","required":["module","depends_on"],"additionalProperties":false,
                "properties":{"module":{"type":"string","minLength":1,"maxLength":256},
                    "depends_on":{"type":"string","minLength":1,"maxLength":256}}
            }}}),
        ),
        definition(
            "reconstruction_contracts",
            "Save bounded module interfaces and ABI/lifetime rules before implementing their functions. Prism hashes the contents. Record unresolved behavior in unknowns; set ready explicitly only when the interface is usable. All contracts in a dependency cycle must be ready before its implementation can be claimed. Release active leases first. Changed contracts invalidate affected source acceptance and consumers, preserving files for merge.",
            &["contracts"],
            json!({"contracts":{"type":"array","minItems":1,"maxItems":32,"items":{
                "type":"object","required":["module","interface","abi"],"additionalProperties":false,
                "properties":{"module":{"type":"string","minLength":1,"maxLength":256},
                    "interface":{"type":"string","maxLength":8192},
                    "abi":{"type":"string","maxLength":8192},
                    "unknowns":{"type":"string","maxLength":8192},
                    "ready":{"type":"boolean"}}
            }}}),
        ),
    ]
}
