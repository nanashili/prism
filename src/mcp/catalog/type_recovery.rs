use serde_json::{Value, json};

pub(super) fn definition() -> Value {
    let base = object(
        &["function", "variable"],
        json!({
            "function":text(32),"variable":text(128)
        }),
    );
    let kind =
        json!({"type":"string","enum":["bytes","integer","float","pointer","function_pointer"]});
    let mut value_properties =
        json!({"kind":kind,"width":{"type":"integer","minimum":1,"maximum":256},"pointee":base});
    let value = object(&["kind", "width"], value_properties.clone());
    value_properties["base"] = base.clone();
    value_properties["offset"] = json!({"type":"integer","minimum":0,"maximum":4294967295_u64});
    value_properties["count"] = json!({"type":"integer","minimum":1,"maximum":1000000});
    value_properties["stride"] = json!({"type":"integer","minimum":1,"maximum":4294967295_u64});
    value_properties["target"] = text(32);
    value_properties["site"] = text(512);
    let access = object(
        &["base", "offset", "width", "kind", "site"],
        value_properties,
    );
    let alias = object(
        &["left", "right", "site"],
        json!({"left":base,"right":base,"site":text(512)}),
    );
    let name = object(&["base", "name"], json!({"base":base,"name":text(128)}));
    let prototype = object(
        &["function", "convention", "parameters", "site"],
        json!({
            "function":text(32),
            "convention":{"type":"string","enum":["unknown","cdecl","stdcall","fastcall","thiscall","system_v64","win64","aapcs64"]},
            "parameters":array(value.clone(),64),"returns":value,"site":text(512)
        }),
    );
    super::tool(
        "recover_types",
        "Recover shared layouts and prototypes from agent observations of saved native functions. propose merges evidence with accepted observations, follows explicit same-origin pointer aliases, infers fields, bounded strided arrays and possible vtables. check revalidates a proposal; accept repeats all checks and atomically publishes every consumer only without conflicts. read returns accepted types, optionally for one address. Evidence and site references are agent hypotheses, not independently verified memory accesses. Types accompany batch results; provider pseudocode is not rewritten. Use the exact proposal ID from propose/check. Only one pending proposal is retained per analysis configuration.",
        &["project", "action"],
        json!({
            "project":text(64),
            "action":{"type":"string","enum":["propose","check","accept","read"]},
            "proposal":text(64),"address":text(32),
            "update":{"type":"string","enum":["merge","replace"],"description":"For propose only. Defaults to merge. replace proposes a complete corrected snapshot; include every consumer still supported by the evidence. Acceptance remains atomic for the whole model."},
            "evidence":object(&["pointer_size"],json!({
                "pointer_size":{"type":"integer","enum":[4,8]},
                "accesses":array(access,1024),"aliases":array(alias,512),
                "prototypes":array(prototype,256),"names":array(name,256)
            }))
        }),
    )
}

fn text(maximum: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":maximum})
}

fn object(required: &[&str], properties: Value) -> Value {
    json!({"type":"object","required":required,"properties":properties,"additionalProperties":false})
}

fn array(items: Value, maximum: usize) -> Value {
    json!({"type":"array","maxItems":maximum,"items":items})
}
