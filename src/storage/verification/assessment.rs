use super::model::{
    Assessment, CheckOutcome, CheckResult, ClaimDraft, ClaimScope, ClaimStatus, Confidence,
    Evidence, EvidenceCheck, Predicate, SourceKind,
};

pub(super) fn assess(draft: &ClaimDraft, evidence: &[Evidence]) -> Assessment {
    let checks: Vec<_> = draft
        .checks
        .iter()
        .zip(evidence)
        .map(|(check, evidence)| evaluate(check, evidence))
        .collect();
    let (status, confidence, reason) = conclude(draft, evidence, &checks);
    Assessment {
        status,
        confidence,
        reason: reason.into(),
        checks,
    }
}

fn evaluate(check: &EvidenceCheck, evidence: &Evidence) -> CheckResult {
    let (outcome, reason) = if !evidence.current {
        (
            CheckOutcome::Unresolved,
            "Evidence is stale; capture and cite fresh analysis",
        )
    } else if evidence.kind == SourceKind::Runtime
        && check.pointer != "/reply"
        && !check.pointer.starts_with("/reply/")
    {
        (
            CheckOutcome::Unresolved,
            "Runtime evidence must select the debugger reply; request arguments are not observations",
        )
    } else if let Some(actual) = evidence.data.pointer(&check.pointer) {
        match &check.predicate {
            Predicate::Equals { expected } if actual == expected => (
                CheckOutcome::Supports,
                "Captured value equals the expected value",
            ),
            Predicate::Equals { .. } => (
                CheckOutcome::Contradicts,
                "Captured value differs from the expected value",
            ),
            Predicate::Contains { text }
                if actual.as_str().is_some_and(|value| value.contains(text)) =>
            {
                (
                    CheckOutcome::Supports,
                    "Exact text occurs in the captured result",
                )
            }
            Predicate::Contains { .. } => {
                // A decompiler or provider may omit relevant paths. Absence from
                // a partial result is not proof that behavior is absent.
                (
                    CheckOutcome::Unresolved,
                    "Text is absent or the selected value is not text; incomplete analysis cannot disprove behavior",
                )
            }
        }
    } else {
        (
            CheckOutcome::Unresolved,
            "The JSON pointer is absent from the captured result",
        )
    };
    CheckResult {
        evidence: check.evidence.clone(),
        pointer: check.pointer.clone(),
        outcome,
        reason: reason.into(),
    }
}

fn conclude(
    draft: &ClaimDraft,
    evidence: &[Evidence],
    checks: &[CheckResult],
) -> (ClaimStatus, Confidence, &'static str) {
    if checks
        .iter()
        .any(|check| check.outcome == CheckOutcome::Unresolved)
    {
        return (
            ClaimStatus::Unresolved,
            Confidence::None,
            "Some evidence is stale, missing, or inconclusive",
        );
    }
    if draft.scope == ClaimScope::Interpretation {
        return (
            ClaimStatus::Unresolved,
            Confidence::None,
            "Predicate results do not establish this behavioral interpretation; semantic verification is still required",
        );
    }
    if checks
        .iter()
        .any(|check| check.outcome == CheckOutcome::Contradicts)
    {
        return (
            ClaimStatus::Contradicted,
            Confidence::Direct,
            "Captured values contradict at least one required predicate; confidence describes the contradiction",
        );
    }
    // Only exact structured observations qualify as runtime-observed. Finding
    // a string in debugger output does not establish execution of that behavior.
    let runtime = evidence.iter().zip(&draft.checks).all(|(item, check)| {
        item.kind == SourceKind::Runtime && matches!(check.predicate, Predicate::Equals { .. })
    });
    if runtime {
        return (
            ClaimStatus::RuntimeObserved,
            Confidence::Observed,
            "All declared values were observed in debugger replies for this execution only",
        );
    }
    let exact = draft
        .checks
        .iter()
        .all(|check| matches!(check.predicate, Predicate::Equals { .. }));
    if exact {
        (
            ClaimStatus::Corroborated,
            Confidence::Direct,
            "All declared predicates match captured values; this verifies the predicates, not broader semantic equivalence",
        )
    } else {
        (
            ClaimStatus::Corroborated,
            Confidence::Limited,
            "All declared predicates match; textual occurrences provide limited evidence of behavior",
        )
    }
}
