use super::{Mode, definitions};
use serde_json::Value;

pub fn validate_call(mode: Mode, params: &Value) -> Result<(), String> {
    let name = params["name"].as_str().ok_or("A tool name is required")?;
    let definitions = definitions(mode);
    let tool = definitions["tools"]
        .as_array()
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool["name"].as_str() == Some(name))
        })
        .ok_or_else(|| format!("Unknown tool: {name}"))?;
    let empty = serde_json::json!({});
    let raw_arguments = params.get("arguments").unwrap_or(&empty);
    let arguments = raw_arguments
        .as_object()
        .ok_or("Tool arguments must be an object")?;
    if name == "decompile_batch" {
        super::decompilation_arguments::validate(arguments)?;
    }
    validate_arguments(raw_arguments, &tool["inputSchema"])?;
    if name == "open_function"
        && (arguments.contains_key("address") == arguments.contains_key("function"))
    {
        return Err("Supply either function or address".into());
    }
    if name == "rename_function" {
        let name = arguments["new_name"].as_str().ok_or("Missing new name")?;
        let mut characters = name.bytes();
        let valid_start = characters
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
        if !valid_start || !characters.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
            return Err("The new name must be a C identifier".into());
        }
    }
    Ok(())
}

pub fn validate_arguments(arguments: &Value, schema: &Value) -> Result<(), String> {
    let arguments = arguments
        .as_object()
        .ok_or("Tool arguments must be an object")?;
    for required in schema["required"].as_array().into_iter().flatten() {
        let key = required.as_str().ok_or("Invalid tool schema")?;
        if !arguments.contains_key(key) {
            return Err(format!("Missing argument: {key}"));
        }
    }
    for (key, value) in arguments {
        let property = schema["properties"]
            .get(key)
            .ok_or_else(|| format!("Unknown argument: {key}"))?;
        validate_property(key, value, property)?;
    }
    Ok(())
}

fn validate_property(key: &str, value: &Value, schema: &Value) -> Result<(), String> {
    let valid_type = match schema["type"].as_str() {
        Some("string") => value.is_string(),
        Some("integer") => value.as_u64().is_some(),
        Some("boolean") => value.is_boolean(),
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        _ => false,
    };
    if !valid_type {
        return Err(format!("Invalid type for {key}"));
    }
    if let Some(items) = value.as_array() {
        validate_array(key, items, schema)?;
    }
    if value.is_object() && schema.get("properties").is_some() {
        validate_arguments(value, schema).map_err(|error| format!("{key}: {error}"))?;
    }
    if let Some(options) = schema["enum"].as_array()
        && !options.contains(value)
    {
        return Err(format!("Unsupported value for {key}"));
    }
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as u64;
        if schema["minLength"].as_u64().is_some_and(|min| length < min)
            || schema["maxLength"].as_u64().is_some_and(|max| length > max)
        {
            return Err(format!("Invalid length for {key}"));
        }
    }
    if let Some(number) = value.as_u64()
        && (schema["minimum"].as_u64().is_some_and(|min| number < min)
            || schema["maximum"].as_u64().is_some_and(|max| number > max))
    {
        return Err(format!("Value outside the supported range for {key}"));
    }
    Ok(())
}

fn validate_array(key: &str, items: &[Value], schema: &Value) -> Result<(), String> {
    let count = items.len() as u64;
    if schema["minItems"].as_u64().is_some_and(|min| count < min)
        || schema["maxItems"].as_u64().is_some_and(|max| count > max)
    {
        return Err(format!("Invalid item count for {key}"));
    }

    let item_schema = schema
        .get("items")
        .ok_or_else(|| format!("Missing item schema for {key}"))?;
    for (index, item) in items.iter().enumerate() {
        validate_property(&format!("{key}[{index}]"), item, item_schema)?;
    }
    Ok(())
}
