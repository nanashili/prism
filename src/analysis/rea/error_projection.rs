use serde_json::Value;

pub(super) const MAX_ERROR_DETAILS_BYTES: usize = 64 * 1024;

pub(super) fn valid_error_projection(error: &Value) -> bool {
    let Some(fields) = error.as_object() else {
        return false;
    };
    if !fields.keys().all(|key| {
        matches!(
            key.as_str(),
            "code" | "category" | "message" | "retryable" | "remediation" | "details"
        )
    }) {
        return false;
    }

    let Some(code) = error["code"].as_str() else {
        return false;
    };
    let Some(category) = error["category"].as_str() else {
        return false;
    };
    if !known_code(code)
        || !known_category(category)
        || !non_empty_string(&error["message"])
        || !error["retryable"].is_boolean()
        || !valid_remediation(&error["remediation"])
        || !valid_details(code, error.get("details"))
    {
        return false;
    }

    serde_json::to_vec(error).is_ok_and(|details| details.len() <= MAX_ERROR_DETAILS_BYTES)
}

fn known_code(code: &str) -> bool {
    matches!(
        code,
        "invalid_request"
            | "access_denied"
            | "artifact_changed"
            | "unreadable_output"
            | "capability_unavailable"
            | "unsupported_target"
            | "provider_unavailable"
            | "provider_timeout"
            | "resource_constraint"
            | "cancelled"
            | "artifact_integrity_mismatch"
            | "artifact_operation_failed"
            | "evidence_integrity_mismatch"
            | "truncated"
            | "process_capture_failed"
            | "cleanup_incomplete"
            | "revision_conflict"
            | "configuration_invalid"
            | "target_unavailable"
            | "execution_failure"
    )
}

fn known_category(category: &str) -> bool {
    matches!(
        category,
        "invalid_input"
            | "unsupported_provider"
            | "unsupported_target"
            | "integrity_mismatch"
            | "truncated"
            | "cancelled"
            | "timeout"
            | "resource_constraint"
            | "unavailable"
            | "execution_failure"
    )
}

fn valid_remediation(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|fields| fields.len() == 1 && non_empty_string(&value["action"]))
}

fn valid_details(code: &str, details: Option<&Value>) -> bool {
    let Some(details) = details else {
        return code != "artifact_integrity_mismatch";
    };
    let Some(fields) = details.as_object() else {
        return false;
    };

    if code != "artifact_integrity_mismatch" {
        return true;
    }

    let required = [
        "logical_path",
        "declared_sha256",
        "calculated_sha256",
        "unpacked",
    ];
    fields.len() == required.len()
        && required.iter().all(|field| fields.contains_key(*field))
        && details["logical_path"].is_string()
        && nullable_string(&details["declared_sha256"])
        && nullable_string(&details["calculated_sha256"])
        && details["unpacked"].is_boolean()
}

fn non_empty_string(value: &Value) -> bool {
    value.as_str().is_some_and(|text| !text.is_empty())
}

fn nullable_string(value: &Value) -> bool {
    value.is_null() || value.is_string()
}
