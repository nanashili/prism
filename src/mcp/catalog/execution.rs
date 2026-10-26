use super::tool;
use serde_json::{Value, json};

pub(super) fn definition() -> Value {
    tool(
        "execution_evidence",
        concat!(
            "Correlate workflow execution observations with the open native binary's static graph. ",
            "Actions: import a version-1 trace, list metadata, read a trace by trace_id, remove a trace, ",
            "functions by trace_id with optional offset (300/page, exact observed entries only), ",
            "or graph for an indexed function address (includes fresh dynamic annotations). ",
            "capture_debugger records one stopped-thread PC sample from the open target explicitly launched in ",
            "the UI; provide trace_id, workflow, runtime_version, thread_id and mapping. It never launches, ",
            "attaches, resumes, steps or evaluates a target. Attached sessions lack verified target identity ",
            "and cannot be sampled. Debugger samples are incomplete evidence. ",
            "Trace fields: version=1, id, workflow, source (debugger/frida_stalker/imported), runtime_version, ",
            "captured_at, target_sha256, mapping {runtime_base,static_base,size}, complete, events. ",
            "Each event has kind (instruction/block/function/call/sample), hexadecimal address, thread_id, ",
            "count (1..1000000000), and target only for calls. Mapping size is bytes; bases are hexadecimal. ",
            "Limits: 10000 events and 2 MiB per immutable trace; 16 traces per target. Unmapped targets stay ",
            "unresolved. Counts are observations, not inferred coverage or exact invocation counts. ",
            "Observed calls can identify targets of indirect calls when compared with static disassembly; ",
            "they do not establish that a call was indirect. Absence never proves a function is unreachable. ",
            "Use scripts/trace-frida.py for optional external capture. Run or instrument executable targets ",
            "only with explicit user authorization in an appropriately isolated environment."
        ),
        &["action"],
        json!({
            "action":{"type":"string","enum":["import","list","read","remove","functions","graph","capture_debugger"]},
            "offset":{"type":"integer","minimum":0,"maximum":1000000},
            "trace":{"type":"object"},
            "trace_id":{"type":"string","minLength":1,"maxLength":128},
            "workflow":{"type":"string","minLength":1,"maxLength":1024},
            "address":{"type":"string","minLength":1,"maxLength":18},
            "runtime_version":{"type":"string","minLength":1,"maxLength":256},
            "thread_id":{"type":"integer","minimum":1},
            "mapping":{"type":"object","required":["runtime_base","static_base","size"],"properties":{
                "runtime_base":{"type":"string","minLength":1,"maxLength":18},
                "static_base":{"type":"string","minLength":1,"maxLength":18},
                "size":{"type":"integer","minimum":1}
            }}
        }),
    )
}
