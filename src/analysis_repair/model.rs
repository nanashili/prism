use crate::error::{Result, failure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Raw observations come only from saved artifacts or the provider capture path.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Observation {
    pub address: String,
    pub pseudocode: Option<String>,
    pub assembly: Option<Value>,
    pub dossier: Option<Value>,
    pub errors: Vec<String>,
}

impl Observation {
    pub(crate) fn digest(&self) -> Result<String> {
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Plan {
    /// A client-supplied specialist task/session reference, not proof of delegation.
    pub specialist: String,
    pub diagnosis: String,
    pub adjustment: Adjustment,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Adjustment {
    RefreshDecompilation,
    InspectControlFlow,
    CallingConvention { convention: String },
    FunctionBoundary { end: String },
    TypeAssumption { declaration: String },
    JumpTable { description: String },
}

impl Adjustment {
    pub(crate) fn blocked_reason(&self) -> Option<&'static str> {
        match self {
            Self::RefreshDecompilation | Self::InspectControlFlow => None,
            _ => Some(
                "The provider integration has no reversible native override API for this adjustment. The proposal remains an interpretation; no provider mutation or rerun was performed.",
            ),
        }
    }
}

impl Plan {
    pub(crate) fn parse(value: &Value) -> Result<Self> {
        let plan: Self = serde_json::from_value(value.clone())?;
        for (label, text, limit) in [
            ("specialist reference", plan.specialist.as_str(), 256),
            ("specialist diagnosis", plan.diagnosis.as_str(), 8192),
        ] {
            if text.trim().is_empty() || text.len() > limit {
                return Err(failure(format!(
                    "Supply a nonblank {label} of at most {limit} bytes"
                )));
            }
        }
        if serde_json::to_vec(&plan.adjustment)?.len() > 4096 {
            return Err(failure("An analysis adjustment must fit within 4096 bytes"));
        }
        match &plan.adjustment {
            Adjustment::CallingConvention { convention } if convention.trim().is_empty() => {
                return Err(failure(
                    "A calling convention proposal requires a convention",
                ));
            }
            Adjustment::FunctionBoundary { end }
                if crate::analysis::graph::address_value(end).is_none() =>
            {
                return Err(failure(
                    "A function boundary proposal requires a valid end address",
                ));
            }
            Adjustment::TypeAssumption { declaration } if declaration.trim().is_empty() => {
                return Err(failure("A type proposal requires a declaration"));
            }
            Adjustment::JumpTable { description } if description.trim().is_empty() => {
                return Err(failure("A jump table proposal requires a description"));
            }
            _ => {}
        }
        Ok(plan)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Concern {
    SuspiciousOutput,
    UnresolvedJumpTable,
    FunctionBoundary,
    CallingConvention,
    TypeAssumption,
    MissingEvidence,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Signal {
    pub concern: Concern,
    pub evidence: String,
}
