//! Interpret only structured fields exposed by the native function dossier.
use super::diagnostics::{add, output_signals};
use super::model::{Concern, Signal};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn diagnose(dossier: &Value, signals: &mut Vec<Signal>) {
    inspect_limitations(dossier, signals);
    let boundary = &dossier["native_api"];
    if boundary.is_null() || boundary["available"] == false {
        add(
            signals,
            Concern::MissingEvidence,
            "Native ABI and type evidence is unavailable",
        );
        return;
    }
    if boundary["available"] != true {
        return;
    }

    inspect_limitations(boundary, signals);
    if let Some(convention) = boundary["calling_convention"].as_str() {
        let convention = convention.to_ascii_lowercase();
        if convention.contains("unknown") || convention.contains("unresolved") {
            add(
                signals,
                Concern::CallingConvention,
                "Provider reports an unresolved calling convention",
            );
        }
    }
    inspect_jump_tables(boundary, signals);
    inspect_parameter_storage(boundary, signals);
}

fn inspect_limitations(value: &Value, signals: &mut Vec<Signal>) {
    for text in value["limitations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        output_signals(text, signals);
        let lower = text.to_ascii_lowercase();
        if lower.contains("unresolved")
            && (lower.contains("indirect call") || lower.contains("indirect jump"))
        {
            add(signals, Concern::MissingEvidence, text);
        }
    }
}

fn inspect_jump_tables(boundary: &Value, signals: &mut Vec<Signal>) {
    for table in boundary["jump_tables"].as_array().into_iter().flatten() {
        inspect_limitations(table, signals);
        let mappings = table["mappings"].as_array();
        let unresolved = mappings.is_none_or(Vec::is_empty)
            || mappings
                .into_iter()
                .flatten()
                .any(|mapping| mapping["case_value"].is_null());
        if unresolved {
            add(
                signals,
                Concern::UnresolvedJumpTable,
                "Provider jump table has missing case values or mappings",
            );
        }
    }
}

fn inspect_parameter_storage(boundary: &Value, signals: &mut Vec<Signal>) {
    // Return and parameter registers may legitimately coincide. Compare only
    // parameters with an identical provider storage description, without guessing aliases.
    let mut declarations = BTreeMap::new();
    for parameter in boundary["parameters"].as_array().into_iter().flatten() {
        let (Some(storage), Some(data_type)) = (
            parameter["storage"].as_str(),
            parameter["data_type"].as_str(),
        ) else {
            continue;
        };
        if storage.is_empty() {
            continue;
        }
        if declarations
            .insert(storage, data_type)
            .is_some_and(|previous| previous != data_type)
        {
            add(
                signals,
                Concern::TypeAssumption,
                "Parameters assign incompatible declarations to identical storage",
            );
        }
    }
}
