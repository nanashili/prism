use super::types::{Budget, Graph, Kind, Usage};
use crate::error::{Result, failure};
use crate::model::Procedure;
use crate::storage::Store;
use std::collections::BTreeMap;

pub(super) fn create(
    store: &Store,
    project: &str,
    objective: &str,
    budget: Budget,
) -> Result<Graph> {
    let objective = objective.trim();
    if objective.is_empty() || objective.chars().count() > 4000 {
        return Err(failure(
            "An investigation objective requires 1–4000 characters",
        ));
    }
    budget.validate()?;

    let candidates = candidates(store, project, objective, budget.functions.min(8))?;
    let mut graph = prepare_graph(project, objective, budget)?;
    seed_tasks(&mut graph, candidates)?;
    Ok(graph)
}

fn prepare_graph(project: &str, objective: &str, budget: Budget) -> Result<Graph> {
    let mut identity = [0u8; 16];
    getrandom::fill(&mut identity)
        .map_err(|error| failure(format!("Creating an investigation ID: {error}")))?;
    let id = identity.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(Graph {
        version: 1,
        id,
        project: project.into(),
        objective: objective.into(),
        revision: 0,
        paused: false,
        budget,
        usage: Usage::default(),
        tasks: Vec::new(),
    })
}

fn seed_tasks(graph: &mut Graph, candidates: Vec<Procedure>) -> Result<()> {
    let survey = graph.add_task(
        Kind::Survey,
        "Identify entry points and available evidence".into(),
        vec![],
        0,
    )?;
    let locate = graph.add_task(
        Kind::Locate,
        "Locate anchors for the objective".into(),
        vec![survey],
        0,
    )?;
    let mut dependencies = vec![survey, locate];
    for candidate in candidates {
        let name: String = candidate.name.chars().take(160).collect();
        let id = graph.add_task(
            Kind::Inspect {
                address: candidate.address.clone(),
            },
            format!("Investigate candidate {} ({})", name, candidate.address),
            vec![locate],
            0,
        )?;
        dependencies.push(id);
    }
    graph.add_task(
        Kind::Synthesize,
        "Explain the supported behavior and remaining limits".into(),
        dependencies,
        0,
    )?;
    Ok(())
}

fn candidates(store: &Store, project: &str, objective: &str, limit: u32) -> Result<Vec<Procedure>> {
    let mut ranked = BTreeMap::new();
    for term in search_terms(objective) {
        for procedure in store.functions(project, &term, 0)? {
            let entry = ranked
                .entry(procedure.address.clone())
                .or_insert((0u32, procedure));
            entry.0 += 1;
        }
    }
    let mut ranked: Vec<_> = ranked.into_values().collect();
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.address.cmp(&right.1.address))
    });
    Ok(ranked
        .into_iter()
        .take(usize::try_from(limit)?)
        .map(|(_, procedure)| procedure)
        .collect())
}

fn search_terms(objective: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for word in objective.split(|character: char| !character.is_alphanumeric()) {
        let word = word.to_lowercase();
        if word.len() < 3 || STOP_WORDS.contains(&word.as_str()) {
            continue;
        }
        // Common inflections are useful symbol anchors, never proof of behavior.
        let term = word.strip_suffix('s').unwrap_or(&word).to_owned();
        if !terms.contains(&term) {
            terms.push(term);
        }
        if terms.len() == 12 {
            break;
        }
    }
    if terms.is_empty() {
        terms.push("main".into());
    }
    terms
}

const STOP_WORDS: &[&str] = &[
    "understand",
    "application",
    "this",
    "that",
    "how",
    "the",
    "and",
    "for",
    "with",
    "from",
    "what",
    "does",
    "its",
    "into",
    "which",
    "program",
    "explain",
    "investigate",
];
