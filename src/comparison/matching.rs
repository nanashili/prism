use super::{Ambiguity, Annotations, Fingerprint, Match, Method, Report, SignatureScope, Snapshot};
use crate::error::{Result, failure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn compare(source: Snapshot, target: Snapshot) -> Result<Report> {
    if source.architecture != target.architecture {
        return Err(failure(
            "Cross-architecture function matching is not supported",
        ));
    }
    let mut correspondences = Correspondences::default();
    correspondences.match_signatures(&source.functions, &target.functions);
    correspondences.match_call_graph(&source.functions, &target.functions);
    let ambiguous = correspondences.group_ambiguities(&source.functions, &target.functions);
    correspondences
        .matches
        .sort_by(|a, b| a.source.address.cmp(&b.source.address));

    Ok(Report {
        source_sha256: source.project.target.sha256,
        target_sha256: target.project.target.sha256,
        source_name: source.project.target.name,
        target_name: target.project.target.name,
        architecture: source.architecture,
        matches: correspondences.matches,
        ambiguous,
        source_only: unmatched(source.functions, &correspondences.source),
        target_only: unmatched(target.functions, &correspondences.target),
        source_unknown: source.unknown,
        target_unknown: target.unknown,
        annotations: Annotations {
            labels: source.project.renamed,
            notes: source.project.notes,
            bookmarks: source.project.bookmarks,
        },
    })
}

#[derive(Default)]
struct Correspondences {
    source: BTreeSet<String>,
    target: BTreeSet<String>,
    matches: Vec<Match>,
}

impl Correspondences {
    fn accept(&mut self, item: Match) {
        self.source.insert(item.source.address.clone());
        self.target.insert(item.target.address.clone());
        self.matches.push(item);
    }

    fn match_signatures(&mut self, source: &[Fingerprint], target: &[Fingerprint]) {
        for method in [
            Method::ExactBytes,
            Method::NormalizedInstructions,
            Method::Structure,
        ] {
            let left = groups(source, &self.source, method);
            let right = groups(target, &self.target, method);
            for (key, before) in left {
                if let ([before], Some(after)) = (before.as_slice(), right.get(&key))
                    && let [after] = after.as_slice()
                {
                    self.accept(pair(before, after, method));
                }
            }
        }
    }

    fn match_call_graph(&mut self, source: &[Fingerprint], target: &[Fingerprint]) {
        // Established correspondences provide address-independent callee anchors.
        // Bound propagation rather than walking long call chains quadratically.
        for _ in 0..8 {
            let anchors = self
                .matches
                .iter()
                .map(|item| (item.source.address.as_str(), item.target.address.as_str()))
                .collect();
            let left = anchored_groups(source, &self.source, Some(&anchors));
            let right = anchored_groups(target, &self.target, None);
            let mut additions = Vec::new();
            for (key, before) in left {
                if let ([before], Some(after)) = (before.as_slice(), right.get(&key))
                    && let [after] = after.as_slice()
                {
                    additions.push(pair(before, after, Method::CallGraph));
                }
            }
            if additions.is_empty() {
                break;
            }
            for item in additions {
                self.accept(item);
            }
        }
    }

    fn group_ambiguities(
        &mut self,
        source: &[Fingerprint],
        target: &[Fingerprint],
    ) -> Vec<Ambiguity> {
        let left = groups(source, &self.source, Method::Structure);
        let right = groups(target, &self.target, Method::Structure);
        let mut ambiguous = Vec::new();
        for (key, before) in left {
            if let Some(after) = right.get(&key) {
                ambiguous.push(Ambiguity {
                    source: before.iter().map(|item| item.function.clone()).collect(),
                    target: after.iter().map(|item| item.function.clone()).collect(),
                });
                self.source
                    .extend(before.iter().map(|item| item.function.address.clone()));
                self.target
                    .extend(after.iter().map(|item| item.function.address.clone()));
            }
        }
        ambiguous
    }
}

fn unmatched(
    functions: Vec<Fingerprint>,
    covered: &BTreeSet<String>,
) -> Vec<crate::model::Procedure> {
    functions
        .into_iter()
        .filter(|item| !covered.contains(&item.function.address))
        .map(|item| item.function)
        .collect()
}

fn pair(source: &Fingerprint, target: &Fingerprint, method: Method) -> Match {
    Match {
        source: source.function.clone(),
        target: target.function.clone(),
        method,
        bytes_equal: source.bytes == target.bytes,
        source_scope: source.scope,
        target_scope: target.scope,
    }
}

fn groups<'a>(
    functions: &'a [Fingerprint],
    matched: &BTreeSet<String>,
    method: Method,
) -> BTreeMap<(SignatureScope, &'a str), Vec<&'a Fingerprint>> {
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for item in functions
        .iter()
        .filter(|item| !matched.contains(&item.function.address))
    {
        let key = match method {
            Method::ExactBytes => &item.bytes,
            Method::NormalizedInstructions => &item.normalized,
            Method::Structure | Method::CallGraph => &item.structure,
        };
        groups
            .entry((item.scope, key.as_str()))
            .or_default()
            .push(item);
    }
    groups
}

fn anchored_groups<'a>(
    functions: &'a [Fingerprint],
    matched: &BTreeSet<String>,
    anchors: Option<&BTreeMap<&str, &str>>,
) -> BTreeMap<String, Vec<&'a Fingerprint>> {
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for item in functions
        .iter()
        .filter(|item| !matched.contains(&item.function.address))
    {
        let mut callees: Vec<_> = item
            .callees
            .iter()
            .filter_map(|address| match anchors {
                Some(anchors) => anchors.get(address.as_str()).copied(),
                None if matched.contains(address) => Some(address.as_str()),
                None => None,
            })
            .collect();
        if callees.is_empty() {
            continue;
        }
        callees.sort();
        callees.dedup();
        groups
            .entry(format!("{:?}:{}:{callees:?}", item.scope, item.structure))
            .or_default()
            .push(item);
    }
    groups
}
