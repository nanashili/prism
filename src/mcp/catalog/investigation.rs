use super::tool;
use serde_json::{Value, json};

pub(super) fn definition() -> Value {
    let budget = json!({
        "type":"object", "required":["steps","functions","depth","evidence_bytes"],
        "properties":{
            "steps":{"type":"integer","minimum":1,"maximum":1000},
            "functions":{"type":"integer","minimum":1,"maximum":256},
            "depth":{"type":"integer","minimum":0,"maximum":16},
            "evidence_bytes":{"type":"integer","minimum":1024,"maximum":1048576}
        },"additionalProperties":false
    });
    let address = json!({"type":"string","minLength":1,"maxLength":1024});
    tool(
        "investigation",
        "Prism-owned durable investigation task graph, separate from the agent's ACP plan. start builds a graph from an objective and indexed candidates; list/status recover saved work after restart. next persistently claims the highest-priority ready task and returns instructions plus task/attempt; repeated next recovers the same unfinished claim without another budget charge. Execute that task with read-only evidence tools, then complete with a report or block with a reason. report.functions adds indexed function leads; report.unresolved adds relationships (from, optional to, question, evidence) for Prism to schedule before synthesis. Use exact saved addresses. Never complete on unavailable evidence. retry reopens a blocked task after its cause changes; pause/resume controls planning. budget replaces all limits. Budgets bound planner claims, function inspection attempts, relationship depth and saved reports, not provider billing or unrelated tool calls. Depth/step/function limits leave pending tasks visible; raise limits or narrow the investigation explicitly. Every mutation is saved locally under the input hash before returning. No analysis or reconstruction starts implicitly.",
        &["action"],
        json!({
            "action":{"type":"string","enum":["start","list","status","next","complete","block","retry","pause","resume","budget"]},
            "investigation":{"type":"string","minLength":32,"maxLength":32},
            "objective":{"type":"string","minLength":1,"maxLength":4000},
            "offset":{"type":"integer","minimum":0,"maximum":1000000},
            "task":{"type":"integer","minimum":1,"maximum":256},
            "attempt":{"type":"integer","minimum":1,"maximum":1000},
            "reason":{"type":"string","minLength":1,"maxLength":4000},
            "budget":budget,
            "report":{"type":"object","required":["findings"],"additionalProperties":false,"properties":{
                "findings":{"type":"string","minLength":1,"maxLength":65536},
                "functions":{"type":"array","maxItems":16,"items":address},
                "unresolved":{"type":"array","maxItems":16,"items":{
                    "type":"object","required":["from","question","evidence"],"additionalProperties":false,
                    "properties":{
                        "from":address,"to":address,
                        "question":{"type":"string","minLength":1,"maxLength":2000},
                        "evidence":{"type":"string","minLength":1,"maxLength":8000}
                    }
                }}
            }}
        }),
    )
}
