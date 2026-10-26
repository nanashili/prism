use serde_json::{Value, json};

pub(super) fn definition() -> Value {
    super::tool(
        "analysis_repair",
        "Investigate suspicious native analysis in a reversible evidence overlay. scan captures one indexed function's saved pseudocode and requested assembly/CFG and returns a specialist delegation task. Delegate with your agent's own specialist facility, then propose its diagnosis with its real task reference; Prism does not spawn or authenticate specialist agents. rerun bypasses Prism caches and records a new provider request with a before/after comparison; provider analysis caches may still be reused. At most two attempts per overlay, using configured provider request timeouts. Native ABI/type/boundary/jump-table overrides are explicitly capability-blocked. evidence reads baseline or one attempt. discard deactivates the overlay and retains its audit log. No action replaces authoritative artifacts or verifies behavior.",
        &["action"],
        json!({
            "action":{"type":"string","enum":["scan","propose","rerun","status","evidence","discard"]},
            "address":{"type":"string","minLength":1,"maxLength":64},
            "repair":{"type":"string","minLength":64,"maxLength":64},
            "attempt":{"type":"integer","minimum":1,"maximum":2},
            "plan":{
                "type":"object","additionalProperties":false,
                "required":["specialist","diagnosis","adjustment"],
                "properties":{
                    "specialist":{"type":"string","minLength":1,"maxLength":256},
                    "diagnosis":{"type":"string","minLength":1,"maxLength":8192},
                    "adjustment":{
                        "type":"object","additionalProperties":false,"required":["kind"],
                        "properties":{
                            "kind":{"type":"string","enum":["refresh_decompilation","inspect_control_flow","calling_convention","function_boundary","type_assumption","jump_table"]},
                            "convention":{"type":"string","maxLength":1024},
                            "end":{"type":"string","maxLength":64},
                            "declaration":{"type":"string","maxLength":2048},
                            "description":{"type":"string","maxLength":2048}
                        }
                    }
                }
            }
        }),
    )
}
