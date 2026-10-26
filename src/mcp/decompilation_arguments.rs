use serde_json::{Map, Value};

/// Maximum number of indexed addresses accepted in one durable job selection.
pub const MAX_DURABLE_ADDRESSES: usize = 16_384;

/// Validates the shared protocol and saved-selection address-count bound.
///
/// # Errors
/// Returns an error when the selection is empty or exceeds [`MAX_DURABLE_ADDRESSES`].
pub fn validate_durable_address_count(count: usize) -> Result<(), crate::Error> {
    if !(1..=MAX_DURABLE_ADDRESSES).contains(&count) {
        return Err(format!(
            "Supply 1 to {MAX_DURABLE_ADDRESSES} indexed addresses for a durable selection"
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate(arguments: &Map<String, Value>) -> Result<(), crate::Error> {
    let action = arguments.get("action").and_then(Value::as_str);
    let allowed: &[&str] = match action {
        None => &["procedures", "offset"],
        Some("start") => &["action", "addresses", "filter"],
        Some("list") => &["action", "offset", "limit"],
        Some("results") => &["action", "job", "offset", "limit", "include_data"],
        Some("status" | "resume" | "pause" | "cancel" | "retry_failed") => &["action", "job"],
        Some(_) => return Err("Unsupported decompilation action".into()),
    };
    for key in arguments.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(
                format!("Argument {key} is not supported by this decompilation action").into(),
            );
        }
    }

    if action.is_none() && !arguments.contains_key("procedures") {
        return Err("Supply procedures for a selection or action=start for a durable job".into());
    }
    if action == Some("start")
        && arguments.contains_key("addresses")
        && arguments.contains_key("filter")
    {
        return Err("Supply addresses or filter, not both".into());
    }
    if action == Some("start")
        && let Some(addresses) = arguments.get("addresses").and_then(Value::as_array)
    {
        validate_durable_address_count(addresses.len())?;
    }
    if action.is_some_and(|action| !matches!(action, "start" | "list")) {
        let job = arguments
            .get("job")
            .and_then(Value::as_str)
            .ok_or("A job ID is required")?;
        if job.len() != 64 || !job.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("A valid decompilation job ID is required".into());
        }
    }
    Ok(())
}
