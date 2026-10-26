//! Evidence documents and explainable retrieval for function behavior searches.

pub(crate) mod structure;
mod text;

use crate::error::{Result, failure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) use text::Query;

/// One function's observed evidence; descriptions and structural signatures are optional.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Document {
    pub address: String,
    pub name: String,
    pub summary: String,
    pub notes: String,
    pub imports: Vec<String>,
    pub constants: Vec<String>,
    pub callers: Vec<String>,
    pub control_flow: Vec<String>,
    pub decompiler: String,
    pub structure_space: Option<String>,
    pub structure: Vec<Feature>,
}

/// A weighted native structural feature; this is a retrieval heuristic.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Feature {
    pub hash: u32,
    pub weight: f64,
}

/// The contribution of one evidence source to a ranked result.
#[derive(Debug, Serialize)]
pub(crate) struct Explanation {
    pub source: &'static str,
    pub matched_terms: Vec<String>,
    pub excerpt: String,
}

/// A search score is a retrieval heuristic, never a probability of correct behavior.
#[derive(Debug, Serialize)]
pub(crate) struct Hit {
    pub address: String,
    pub name: String,
    pub score: f64,
    pub evidence: Vec<Explanation>,
    pub structural: Option<StructuralMatch>,
}

#[derive(Debug, Serialize)]
pub(crate) struct StructuralMatch {
    pub exemplar: String,
    pub cosine: f64,
    pub seed_relevance: f64,
    pub score: f64,
    pub shared_features: usize,
}

impl Document {
    pub(crate) fn validate(&self) -> Result<()> {
        if super::graph::address_value(&self.address).is_none() {
            return Err(failure(
                "Search documents require a native function address",
            ));
        }
        if serde_json::to_vec(self)?.len() > 512 * 1024 {
            return Err(failure("A function search document exceeds 512 KiB"));
        }
        if self.structure.len() > 16_384 {
            return Err(failure(
                "A native structural signature exceeds 16384 features",
            ));
        }
        if !self.structure.is_empty()
            && !self
                .structure_space
                .as_ref()
                .is_some_and(|space| !space.is_empty() && space.len() <= 256)
        {
            return Err(failure(
                "native structural signatures require a versioned configuration identity",
            ));
        }

        let mut hashes = BTreeSet::new();
        for feature in &self.structure {
            if !feature.weight.is_finite() || feature.weight <= 0.0 {
                return Err(failure(
                    "native structural coefficients must be finite and positive",
                ));
            }
            if !hashes.insert(feature.hash) {
                return Err(failure(
                    "A native structural signature repeats a feature hash",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn fields(&self) -> Vec<(&'static str, String, f64)> {
        vec![
            ("description", self.summary.clone(), 1.0),
            ("notes", self.notes.clone(), 1.0),
            ("imports/callees", self.imports.join(" "), 0.8),
            ("constants", self.constants.join(" "), 0.8),
            ("callers", self.callers.join(" "), 0.5),
            ("control_flow", self.control_flow.join(" "), 0.6),
            ("decompiler", text::body(&self.decompiler).to_owned(), 0.9),
            ("name", self.name.clone(), 0.2),
        ]
    }

    pub(crate) fn index_terms(&self) -> String {
        let mut terms = BTreeSet::new();
        for (_, value, _) in self.fields() {
            terms.extend(text::terms(&value));
        }
        terms.extend(text::behaviors(self).into_iter().map(str::to_owned));
        terms.into_iter().collect::<Vec<_>>().join(" ")
    }

    pub(crate) fn rank(&self, query: &Query) -> Hit {
        let mut evidence = Vec::new();
        let mut contributions = std::collections::BTreeMap::<String, f64>::new();
        for (source, value, weight) in self.fields() {
            let terms = text::terms(&value);
            let matched: Vec<_> = query.terms.intersection(&terms).cloned().collect();
            if matched.is_empty() {
                continue;
            }
            for term in &matched {
                let contribution = contributions.entry(term.clone()).or_default();
                *contribution = contribution.max(weight);
            }
            evidence.push(Explanation {
                source,
                matched_terms: matched,
                excerpt: text::excerpt(&value, &query.terms),
            });
        }

        for behavior in text::behaviors(self) {
            if query.terms.contains(behavior) {
                contributions.insert(behavior.into(), 1.0);
                evidence.push(Explanation {
                    source: "inferred_behavior",
                    matched_terms: vec![behavior.into()],
                    excerpt: text::behavior_evidence(self, behavior),
                });
            }
        }
        let denominator = u32::try_from(query.terms.len()).unwrap_or(u32::MAX);
        let score = contributions.values().sum::<f64>() / f64::from(denominator.max(1));
        Hit {
            address: self.address.clone(),
            name: self.name.clone(),
            score,
            evidence,
            structural: None,
        }
    }
}
