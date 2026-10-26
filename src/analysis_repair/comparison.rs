use super::{Observation, diagnose};
use crate::error::Result;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(crate) fn compare(baseline: &Observation, candidate: &Observation) -> Result<Value> {
    let before = diagnose(baseline);
    let after = diagnose(candidate);
    let removed: Vec<_> = before
        .iter()
        .filter(|signal| !after.iter().any(|other| other.concern == signal.concern))
        .collect();
    let added: Vec<_> = after
        .iter()
        .filter(|signal| !before.iter().any(|other| other.concern == signal.concern))
        .collect();

    Ok(json!({
        "baseline_digest": baseline.digest()?,
        "candidate_digest": candidate.digest()?,
        "pseudocode": component_change(&baseline.pseudocode, &candidate.pseudocode)?,
        "assembly": component_change(&baseline.assembly, &candidate.assembly)?,
        "dossier": component_change(&baseline.dossier, &candidate.dossier)?,
        "control_flow": component_change(&control_flow(baseline), &control_flow(candidate))?,
        "signals_before": before,
        "signals_after": after,
        "removed_signals": removed,
        "added_signals": added,
        "complete_capture": candidate.errors.is_empty()
            && candidate.pseudocode.is_some()
            && candidate.assembly.is_some()
            && candidate.dossier.is_some(),
        "verdict": "unverified",
        "behavior_verified": false,
        "freshness": "new Prism requests; provider analysis caches may be reused",
        "explanation": "Evidence and diagnostic differences are observations, not proof of a correct repair. The original evidence remains selected."
    }))
}

fn control_flow(observation: &Observation) -> Option<&Value> {
    let dossier = observation.dossier.as_ref()?;
    if dossier["facet_availability"]["cfg"]["available"] == false {
        return None;
    }
    dossier
        .get("basic_blocks")
        .filter(|value| value.as_array().is_some_and(|blocks| !blocks.is_empty()))
}

fn component_change<T: Serialize + PartialEq>(
    before: &Option<T>,
    after: &Option<T>,
) -> Result<Value> {
    let changed = match (before, after) {
        (Some(before), Some(after)) => Some(before != after),
        _ => None,
    };
    Ok(json!({
        "changed": changed,
        "baseline_digest": component_digest(before)?,
        "candidate_digest": component_digest(after)?,
    }))
}

fn component_digest<T: Serialize>(value: &Option<T>) -> Result<Option<String>> {
    value
        .as_ref()
        .map(|value| {
            let bytes = serde_json::to_vec(value)?;
            Ok(format!("{:x}", Sha256::digest(bytes)))
        })
        .transpose()
}
