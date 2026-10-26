use crate::error::{Result, failure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceKind {
    StaticAnalysis,
    Runtime,
}

/// Provenance is assigned by Prism's capture path, never by an agent argument.
pub(crate) struct EvidenceSource<'a> {
    pub key: &'a str,
    pub subject: &'a str,
    pub kind: SourceKind,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClaimDraft {
    pub(super) statement: String,
    pub(super) scope: ClaimScope,
    pub(super) checks: Vec<EvidenceCheck>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum ClaimScope {
    /// The statement describes exactly the supplied machine-checkable predicates.
    Predicate,
    /// Behavioral interpretation requires semantic review beyond matching predicates.
    Interpretation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EvidenceCheck {
    pub evidence: String,
    pub pointer: String,
    pub predicate: Predicate,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Predicate {
    Equals { expected: Value },
    Contains { text: String },
}

impl ClaimDraft {
    pub(super) fn validate(&self) -> Result<()> {
        if self.statement.trim().is_empty() || self.statement.len() > 4096 {
            return Err(failure("A claim requires a statement of 1–4096 bytes"));
        }
        if self.checks.is_empty() || self.checks.len() > 16 {
            return Err(failure("A claim requires 1–16 evidence checks"));
        }
        if serde_json::to_vec(self)?.len() > 32 * 1024 {
            return Err(failure("A claim must fit within 32 KiB"));
        }
        for check in &self.checks {
            if check.evidence.len() != 64
                || !check.evidence.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(failure("Evidence references must be SHA-256 identifiers"));
            }
            if check.pointer.len() > 1024
                || (!check.pointer.is_empty() && !check.pointer.starts_with('/'))
            {
                return Err(failure("Use a JSON pointer of at most 1024 bytes"));
            }
            match &check.predicate {
                Predicate::Contains { text } if text.trim().is_empty() || text.len() > 4096 => {
                    return Err(failure("A text check requires 1–4096 nonblank bytes"));
                }
                Predicate::Equals { expected } if serde_json::to_vec(expected)?.len() > 4096 => {
                    return Err(failure("An expected value must fit within 4096 bytes"));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ClaimStatus {
    Proposed,
    Corroborated,
    Contradicted,
    RuntimeObserved,
    Unresolved,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Confidence {
    None,
    Limited,
    Direct,
    Observed,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Assessment {
    pub status: ClaimStatus,
    pub confidence: Confidence,
    pub reason: String,
    pub checks: Vec<CheckResult>,
}

impl Assessment {
    pub(super) fn proposed() -> Self {
        Self {
            status: ClaimStatus::Proposed,
            confidence: Confidence::None,
            reason: "Evidence is cited but has not been checked".into(),
            checks: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct CheckResult {
    pub evidence: String,
    pub pointer: String,
    pub outcome: CheckOutcome,
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum CheckOutcome {
    Supports,
    Contradicts,
    Unresolved,
}

#[derive(Debug, Serialize)]
pub(super) struct Claim {
    pub id: i64,
    pub project: String,
    pub draft: ClaimDraft,
    pub assessment: Assessment,
    pub revision: i64,
    pub created: i64,
    pub updated: i64,
}

#[derive(Debug, Serialize)]
pub(super) struct Evidence {
    pub id: String,
    pub source: String,
    pub subject: String,
    pub kind: SourceKind,
    pub current: bool,
    pub digest: String,
    pub data: Value,
    pub captured: i64,
}
