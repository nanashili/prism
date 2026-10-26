use super::model::{Concern, Observation, Signal};
use crate::analysis::graph::address_value;
use serde_json::Value;

/// Signals are leads for a specialist, never proof that a replacement interpretation is correct.
pub(crate) fn diagnose(observation: &Observation) -> Vec<Signal> {
    let mut signals = Vec::new();
    match observation.pseudocode.as_deref() {
        Some(code) if !code.trim().is_empty() => output_signals(code, &mut signals),
        _ => add(
            &mut signals,
            Concern::SuspiciousOutput,
            "Pseudocode is missing or empty",
        ),
    }
    for error in &observation.errors {
        add(&mut signals, Concern::MissingEvidence, error);
    }
    if let Some(dossier) = &observation.dossier {
        boundary_signals(&observation.address, dossier, &mut signals);
        super::native_signals::diagnose(dossier, &mut signals);
    }
    signals
}

pub(crate) fn pseudocode_signals(code: &str) -> Vec<Signal> {
    let mut signals = Vec::new();
    output_signals(code, &mut signals);
    signals
}

pub(super) fn output_signals(code: &str, signals: &mut Vec<Signal>) {
    for line in code.lines() {
        let lower = line.to_ascii_lowercase();
        let concern = if (lower.contains("jumptable") || lower.contains("jump table"))
            && [
                "unresolved",
                "could not",
                "cannot",
                "failed",
                "incomplete",
                "unknown",
            ]
            .iter()
            .any(|word| lower.contains(word))
        {
            Some(Concern::UnresolvedJumpTable)
        } else if lower.contains("calling convention") || lower.contains("stack pointer") {
            Some(Concern::CallingConvention)
        } else if lower.contains("type conflict")
            || lower.contains("incompatible type")
            || lower.contains("type propagation")
        {
            Some(Concern::TypeAssumption)
        } else if lower.contains("bad instruction")
            || lower.contains("overlapping instruction")
            || lower.contains("function boundary")
            || lower.contains("flow out of")
        {
            Some(Concern::FunctionBoundary)
        } else if lower.contains("warning:")
            || lower.contains("unimplemented")
            || lower.contains("unaff_")
        {
            Some(Concern::SuspiciousOutput)
        } else {
            None
        };
        if let Some(concern) = concern {
            // Keep only one bounded excerpt per concern; provider output may be adversarial.
            let excerpt: String = line.chars().take(240).collect();
            add(signals, concern, &excerpt);
        }
    }
}

fn boundary_signals(address: &str, dossier: &Value, signals: &mut Vec<Signal>) {
    if dossier["facet_availability"]["cfg"]["available"] == false {
        add(
            signals,
            Concern::MissingEvidence,
            dossier["facet_availability"]["cfg"]["reason"]
                .as_str()
                .unwrap_or("Provider reports control-flow evidence is unavailable"),
        );
        return;
    }
    let Some(blocks) = dossier["basic_blocks"].as_array() else {
        add(
            signals,
            Concern::MissingEvidence,
            "Provider did not expose basic blocks",
        );
        return;
    };
    if blocks.len() > 4096 {
        add(
            signals,
            Concern::MissingEvidence,
            "Control-flow inspection exceeds 4096 blocks",
        );
        return;
    }
    if blocks.is_empty() {
        add(
            signals,
            Concern::MissingEvidence,
            "Provider returned no control-flow blocks",
        );
        return;
    }
    let ranges: Option<Vec<_>> = blocks
        .iter()
        .map(|block| {
            Some((
                address_value(block["start"].as_str()?)?,
                address_value(block["end"].as_str()?)?,
            ))
        })
        .collect();
    let Some(mut ranges) = ranges else {
        add(
            signals,
            Concern::FunctionBoundary,
            "Control-flow blocks contain invalid bounds",
        );
        return;
    };
    ranges.sort_unstable();
    let entry = address_value(address);
    if !ranges.iter().any(|(start, _)| Some(*start) == entry) {
        add(
            signals,
            Concern::FunctionBoundary,
            "No basic block begins at the indexed entry point",
        );
    }
    if ranges.iter().any(|(start, end)| start >= end)
        || ranges.windows(2).any(|pair| pair[0].1 > pair[1].0)
    {
        add(
            signals,
            Concern::FunctionBoundary,
            "Control-flow blocks overlap or have inverted bounds",
        );
    }
}

pub(super) fn add(signals: &mut Vec<Signal>, concern: Concern, evidence: &str) {
    if !signals.iter().any(|signal| signal.concern == concern) {
        signals.push(Signal {
            concern,
            evidence: evidence.chars().take(240).collect(),
        });
    }
}
