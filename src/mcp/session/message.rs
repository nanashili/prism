use serde_json::{Value, json};

pub(super) fn validate(request: &Value) -> Result<(), Value> {
    let id = request.get("id");
    if !request.is_object()
        || request["jsonrpc"] != "2.0"
        || !request["method"].is_string()
        || id.is_some_and(|id| !(id.is_string() || id.is_i64() || id.is_u64()))
    {
        return Err(error(Value::Null, -32600, "Invalid JSON-RPC request"));
    }
    if id.is_some()
        && request
            .get("params")
            .is_some_and(|params| !params.is_object())
    {
        return Err(error(
            id.cloned().unwrap_or(Value::Null),
            -32602,
            "Parameters must be an object",
        ));
    }
    Ok(())
}

pub(super) fn tool_result(value: Value) -> Value {
    let structured = if value.is_object() {
        value
    } else {
        json!({"result":value})
    };
    json!({"content":[{"type":"text","text":structured.to_string()}],"structuredContent":structured})
}

pub(super) fn reply(id: Value, result: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":result})
}

pub(super) fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
