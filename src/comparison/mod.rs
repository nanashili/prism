//! Address-independent native function matching and reviewed annotation transfer.

mod capture;
mod listing;
mod matching;
mod signatures;
mod transfer;

pub(crate) use capture::{Capture, SignatureCache};
pub(crate) use matching::compare;
pub(crate) use transfer::transfer;

use crate::model::{Procedure, Project};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Fingerprint {
    pub function: Procedure,
    pub bytes: String,
    pub normalized: String,
    pub structure: String,
    pub callees: Vec<String>,
    pub scope: SignatureScope,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Unknown {
    pub function: Procedure,
    pub reason: String,
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    pub project: Project,
    pub architecture: String,
    pub functions: Vec<Fingerprint>,
    pub unknown: Vec<Unknown>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SignatureScope {
    RecoveredBlocks,
    ListingSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Method {
    ExactBytes,
    NormalizedInstructions,
    Structure,
    CallGraph,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Match {
    pub source: Procedure,
    pub target: Procedure,
    pub method: Method,
    pub bytes_equal: bool,
    pub source_scope: SignatureScope,
    pub target_scope: SignatureScope,
}

/// A compact ambiguity group avoids a quadratic cross product of duplicate functions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Ambiguity {
    pub source: Vec<Procedure>,
    pub target: Vec<Procedure>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Annotations {
    pub labels: BTreeMap<String, String>,
    pub notes: BTreeMap<String, String>,
    pub bookmarks: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Report {
    pub source_sha256: String,
    pub target_sha256: String,
    pub source_name: String,
    pub target_name: String,
    pub architecture: String,
    pub matches: Vec<Match>,
    pub ambiguous: Vec<Ambiguity>,
    /// Unmatched inventory entries are candidates, not proof of added/removed behavior.
    pub source_only: Vec<Procedure>,
    pub target_only: Vec<Procedure>,
    pub source_unknown: Vec<Unknown>,
    pub target_unknown: Vec<Unknown>,
    pub annotations: Annotations,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Selection {
    pub source: String,
    pub target: String,
}

impl Report {
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "source_sha256":self.source_sha256,"target_sha256":self.target_sha256,
            "source_name":self.source_name,"target_name":self.target_name,
            "architecture":self.architecture,"matches":self.matches.len(),
            "changed_bytes":self.matches.iter().filter(|item| !item.bytes_equal).count(),
            "ambiguous_groups":self.ambiguous.len(),
            "source_only":self.source_only.len(),"target_only":self.target_only.len(),
            "source_unknown":self.source_unknown.len(),"target_unknown":self.target_unknown.len(),
            "limitations": [
                "Static candidates; byte equality does not establish semantic equivalence of relocated references.",
                "Listing spans follow the provider inventory, not recovered function bodies. Listing topology is heuristic, not a recovered CFG.",
                "Normalized signatures mask numeric operands; changed constants require review.",
                "Structural matches and duplicate groups require review; no annotations transfer automatically.",
                "Unmatched functions are possible additions/removals; discovery, inlining and extraction gaps remain unknown.",
                "Only confirmed project labels, notes and function bookmarks transfer; no instruction offsets, patches, types or raw evidence are copied."
            ]
        })
    }
}

#[cfg(test)]
mod tests;
