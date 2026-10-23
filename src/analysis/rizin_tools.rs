use crate::error::{Result, failure};
use serde_json::{Value, json};

pub(super) fn definitions() -> Value {
    let tools: Vec<_> = [
        ("list_procedures", "List functions found by Rizin's initial analysis",false),
        ("list_segments", "List binary sections",false),
        ("list_strings", "List strings found in binary sections",false),
        ("procedure_assembly", "Read a function's assembly",true),
        ("procedure_pseudo_code", "Decompile a function using an installed Rizin decompiler plugin",true),
        ("analyze_function", "Read a function's control-flow blocks and call references",true),
        ("procedure_callers", "Read incoming references to a function",true),
        ("procedure_callees", "Read outgoing references from a function",true),
    ].into_iter().map(|(name,description,procedure)| {
        let properties = if procedure { json!({"procedure":{"type":"string","description":"Hexadecimal function address"}}) } else { json!({}) };
        let required: Vec<&str> = if procedure { vec!["procedure"] } else { vec![] };
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false}})
    }).collect();
    json!({"tools":tools})
}

pub(super) fn assembly(function: &Value) -> Result<String> {
    use std::fmt::Write;
    let rows = function["ops"]
        .as_array()
        .ok_or("No assembly is available at this address")?;
    let mut text = String::new();
    for instruction in rows {
        let address = instruction["offset"]
            .as_u64()
            .ok_or("An instruction has no address")?;
        let code = instruction["disasm"]
            .as_str()
            .or_else(|| instruction["opcode"].as_str())
            .ok_or("An instruction has no text")?;
        writeln!(text, "0x{address:x}: {code}")?;
        if text.len() > crate::project::files::TEXT_LIMIT {
            return Err(failure("This function exceeds the assembly display limit"));
        }
    }
    Ok(text)
}

pub(super) fn dossier(blocks: &Value, function: &Value, callers: &Value) -> Result<Value> {
    let blocks = blocks
        .as_array()
        .ok_or("Rizin did not return basic blocks")?;
    let blocks: Vec<_> = blocks.iter().map(|block| {
        let start = block["addr"].as_u64().ok_or("A block has no address")?;
        let size = block["size"].as_u64().ok_or("A block has no size")?;
        let end = start.checked_add(size).ok_or("The block address overflowed")?;
        let successors: Vec<_> = ["jump","fail"].into_iter().filter_map(|key| block[key].as_u64()).map(|address| format!("0x{address:x}")).collect();
        Ok(json!({"start":format!("0x{start:x}"),"end":format!("0x{end:x}"),"successors":successors}))
    }).collect::<Result<_>>()?;
    let mut callers: Vec<_> = callers
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["from"].as_u64())
        .take(1001)
        .map(reference)
        .collect();
    let mut callees: Vec<_> = function["ops"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["type"] == "call")
        .filter_map(|row| row["jump"].as_u64())
        .take(1001)
        .map(reference)
        .collect();
    let limited = callers.len() > 1000 || callees.len() > 1000;
    callers.truncate(1000);
    callees.truncate(1000);
    Ok(json!({
        "basic_blocks": blocks,
        "callers": callers,
        "callees": callees,
        "references_limited": limited,
    }))
}

fn reference(address: u64) -> Value {
    json!({"address":format!("0x{address:x}")})
}
