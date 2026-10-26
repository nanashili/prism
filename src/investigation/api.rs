use super::types::{Budget, Graph, Report};
use crate::error::Result;
use crate::storage::Store;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Start {
        objective: String,
        budget: Option<Budget>,
    },
    List {
        #[serde(default)]
        offset: u32,
    },
    Status {
        investigation: String,
    },
    Next {
        investigation: String,
    },
    Complete {
        investigation: String,
        task: u32,
        attempt: u32,
        report: Report,
    },
    Block {
        investigation: String,
        task: u32,
        attempt: u32,
        reason: String,
    },
    Retry {
        investigation: String,
        task: u32,
    },
    Pause {
        investigation: String,
    },
    Resume {
        investigation: String,
    },
    Budget {
        investigation: String,
        budget: Budget,
    },
}

pub(crate) fn start(
    store: &Store,
    project: &str,
    objective: &str,
    budget: Budget,
) -> Result<Graph> {
    let graph = super::planning::create(store, project, objective, budget)?;
    store.insert_investigation(&graph)?;
    Ok(graph)
}

/// All transitions operate on a private snapshot and publish only after validation succeeds.
#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn call(store: &Store, project: &str, arguments: &Value) -> Result<Value> {
    prism::mcp::validate_call(
        prism::mcp::Mode::Combined,
        &json!({"name":"investigation", "arguments":arguments}),
    )?;
    let request: Request = serde_json::from_value(arguments.clone())?;
    match request {
        Request::Start { objective, budget } => Ok(snapshot(
            &start(store, project, &objective, budget.unwrap_or_default())?,
            None,
        )),
        Request::List { offset } => {
            let graphs = store.investigations(project, offset)?;
            let next_offset = (graphs.len() == 20).then_some(offset + 20);
            let summaries: Vec<_> = graphs.iter().map(summary).collect();
            Ok(json!({"investigations":summaries,"next_offset":next_offset}))
        }
        Request::Status { investigation } => Ok(snapshot(
            &store.investigation(project, &investigation)?,
            None,
        )),
        request => update(store, project, request),
    }
}

fn update(store: &Store, project: &str, request: Request) -> Result<Value> {
    let id = match &request {
        Request::Next { investigation }
        | Request::Complete { investigation, .. }
        | Request::Block { investigation, .. }
        | Request::Retry { investigation, .. }
        | Request::Pause { investigation }
        | Request::Resume { investigation }
        | Request::Budget { investigation, .. } => investigation,
        _ => return Err("This investigation request is not a transition".into()),
    };
    let mut graph = store.investigation(project, id)?;
    let mut next = None;
    match request {
        Request::Next { .. } => next = graph.claim()?,
        Request::Complete {
            task,
            attempt,
            report,
            ..
        } => {
            validate_addresses(store, project, &report)?;
            graph.record(task, attempt, report)?;
        }
        Request::Block {
            task,
            attempt,
            reason,
            ..
        } => graph.block(task, attempt, &reason)?,
        Request::Retry { task, .. } => graph.retry(task)?,
        Request::Pause { .. } => graph.paused = true,
        Request::Resume { .. } => graph.paused = false,
        Request::Budget { budget, .. } => {
            budget.validate()?;
            if budget.steps < graph.usage.steps
                || budget.functions < graph.usage.functions
                || budget.evidence_bytes < graph.usage.evidence_bytes
            {
                return Err("A budget cannot be reduced below recorded usage".into());
            }
            graph.budget = budget;
        }
        _ => return Err("This investigation request is not a transition".into()),
    }
    store.save_investigation(&mut graph)?;
    Ok(snapshot(&graph, next))
}

fn validate_addresses(store: &Store, project: &str, report: &Report) -> Result<()> {
    for address in &report.functions {
        store.require_investigation_function(project, address)?;
    }
    for relationship in &report.unresolved {
        store.require_investigation_function(project, &relationship.from)?;
        if let Some(to) = &relationship.to {
            store.require_investigation_function(project, to)?;
        }
    }
    Ok(())
}

fn summary(graph: &Graph) -> Value {
    json!({
        "id":graph.id,"objective":graph.objective,"revision":graph.revision,
        "status":graph.status(),"completed":graph.completed(),"total":graph.tasks.len(),
        "budget":graph.budget,"usage":graph.usage,
    })
}

fn snapshot(graph: &Graph, next: Option<u32>) -> Value {
    let next = next.and_then(|id| graph.tasks.iter().find(|task| task.id == id)).map(|task| json!({
        "task":task.id,"attempt":task.attempts,"instruction":task.kind.instruction(),"kind":task.kind,
    }));
    json!({"summary":summary(graph),"graph":graph,"next":next})
}
