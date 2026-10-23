use super::batch_state::{Counts, DecompilationItem};
use super::error_projection::{MAX_ERROR_DETAILS_BYTES, valid_error_projection};
use crate::error::{Result, failure};
use serde_json::{Map, Value};

const MAX_PSEUDOCODE_BYTES: usize = 512 * 1024;
const MAX_PSEUDOCODE_JSON_BYTES: usize = 768 * 1024;

pub(super) fn validate_procedure_identity(value: &Value, expected: &str) -> Result<()> {
    let procedure = &value["procedure"];
    match (value["status"].as_str(), procedure["status"].as_str()) {
        (Some("ok" | "error"), Some("resolved")) => {
            validate_resolved_procedure(procedure, expected)
        }
        (Some("error"), Some("unknown")) => validate_unknown_procedure(procedure),
        _ => Err(failure(
            "REA omitted the selected function's procedure identity",
        )),
    }
}

pub(super) fn item(
    mut value: Value,
    expected: &str,
    counts: &mut Counts,
) -> Result<DecompilationItem> {
    validate_procedure_identity(&value, expected)?;
    if value["address"].as_str() != Some(expected) {
        return Err(failure("REA changed the selected function order"));
    }

    let (result, error_details) = match value["status"].as_str() {
        Some("ok") => successful_item(&mut value, counts)?,
        Some("error") => failed_item(&value, counts)?,
        _ => return Err(failure("REA returned an invalid decompilation item status")),
    };

    Ok(DecompilationItem {
        address: expected.into(),
        result,
        error_details,
    })
}

fn successful_item(
    value: &mut Value,
    counts: &mut Counts,
) -> Result<(std::result::Result<String, String>, Option<Value>)> {
    require_fields(
        value,
        &["address", "procedure", "status", "pseudocode"],
        &[],
    )?;
    let text = value
        .get_mut("pseudocode")
        .map(Value::take)
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or("REA omitted successful pseudocode")?;
    if text.is_empty() {
        return Err(failure("REA returned empty successful pseudocode"));
    }
    if text.len() > MAX_PSEUDOCODE_BYTES
        || serde_json::to_vec(&text)?.len() > MAX_PSEUDOCODE_JSON_BYTES
    {
        return Err(failure(
            "REA returned pseudocode larger than the durable artifact limit",
        ));
    }

    counts.succeeded += 1;
    Ok((Ok(text), None))
}

fn failed_item(
    value: &Value,
    counts: &mut Counts,
) -> Result<(std::result::Result<String, String>, Option<Value>)> {
    require_fields(value, &["address", "procedure", "status", "error"], &[])?;
    let error = value
        .get("error")
        .filter(|error| valid_error_projection(error))
        .cloned()
        .ok_or("REA returned an invalid typed decompilation error")?;
    let procedure_error = value.pointer("/procedure/error");
    let message = error["message"]
        .as_str()
        .ok_or("REA omitted the decompilation failure reason")?
        .to_owned();
    let error_details = Some(retain_error_projections(error, procedure_error)?);

    counts.failed += 1;
    Ok((Err(message), error_details))
}

fn retain_error_projections(error: Value, procedure_error: Option<&Value>) -> Result<Value> {
    let mut details = Map::new();
    details.insert("error".into(), error.clone());
    if let Some(procedure_error) = procedure_error.filter(|value| *value != &error) {
        details.insert("procedure_error".into(), procedure_error.clone());
    }
    let details = Value::Object(details);
    if serde_json::to_vec(&details)?.len() > MAX_ERROR_DETAILS_BYTES {
        return Err(failure(
            "REA returned error details larger than the durable artifact limit",
        ));
    }
    Ok(details)
}

fn validate_resolved_procedure(procedure: &Value, expected: &str) -> Result<()> {
    require_fields(procedure, &["status", "address", "name"], &["name_error"])?;
    let actual = procedure["address"]
        .as_str()
        .and_then(crate::analysis::graph::address_value);
    if actual != crate::analysis::graph::address_value(expected)
        || !(procedure["name"].is_null() || procedure["name"].is_string())
        || procedure
            .get("name_error")
            .is_some_and(|error| !valid_error_projection(error))
    {
        return Err(failure(
            "REA resolved a different function entry than the saved selection",
        ));
    }
    Ok(())
}

fn validate_unknown_procedure(procedure: &Value) -> Result<()> {
    require_fields(procedure, &["status", "error"], &[])?;
    if !procedure.get("error").is_some_and(valid_error_projection) {
        return Err(failure(
            "REA returned an invalid procedure resolution error",
        ));
    }
    Ok(())
}

fn require_fields(value: &Value, required: &[&str], optional: &[&str]) -> Result<()> {
    let Some(fields) = value.as_object() else {
        return Err(failure("REA returned an invalid batch result object"));
    };
    if required.iter().all(|field| fields.contains_key(*field))
        && fields
            .keys()
            .all(|key| required.contains(&key.as_str()) || optional.contains(&key.as_str()))
    {
        Ok(())
    } else {
        Err(failure(
            "REA returned missing or unexpected batch result fields",
        ))
    }
}
