use super::{Graph, address_value};
use crate::analysis::execution::{ExecutionTrace, ObservationKind, TraceSource};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize)]
pub(crate) struct TraceEvidence {
    pub trace_id: String,
    pub workflow: String,
    pub source: TraceSource,
    pub runtime_version: String,
    pub captured_at: String,
    pub complete: bool,
    pub function_observed: bool,
    pub blocks: Vec<BlockEvidence>,
    pub calls: Vec<CallEvidence>,
    pub unmapped_observations: usize,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct BlockEvidence {
    pub start: String,
    pub observations: u64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct CallEvidence {
    pub site: String,
    pub target: Option<String>,
    pub runtime_target: String,
    pub observations: u64,
}

impl Graph {
    /// Adds observations without changing static successors, callers, or callees.
    pub(crate) fn annotate(&mut self, traces: &[ExecutionTrace]) {
        let ranges: BTreeMap<_, _> = self
            .blocks
            .iter()
            .filter_map(|block| {
                Some((
                    address_value(&block.start)?,
                    (address_value(&block.end)?, block.start.as_str()),
                ))
            })
            .collect();
        self.dynamic = traces
            .iter()
            .map(|trace| correlate(&ranges, trace))
            .collect();
    }
}

fn correlate(ranges: &BTreeMap<u64, (u64, &str)>, trace: &ExecutionTrace) -> TraceEvidence {
    let mut blocks = BTreeMap::<&str, u64>::new();
    let mut calls = BTreeMap::<(u64, Option<u64>, &str), u64>::new();
    let mut unmapped = 0;

    for observation in &trace.events {
        let site = trace.mapping.static_address(&observation.address);
        if site.is_none() {
            unmapped += 1;
        }
        if let Some(site) = site
            && let Some(block) = block_containing(ranges, site)
        {
            *blocks.entry(block).or_default() += observation.count;
            if let Some(target) = observation.target.as_deref() {
                let static_target = trace.mapping.static_address(target);
                *calls.entry((site, static_target, target)).or_default() += observation.count;
            }
        }

        // Call targets establish execution even when the caller is outside this function.
        if observation.kind == ObservationKind::Call
            && let Some(target) = observation.target.as_deref()
            && let Some(target) = trace.mapping.static_address(target)
            && let Some(block) = block_containing(ranges, target)
        {
            *blocks.entry(block).or_default() += observation.count;
        }
    }

    TraceEvidence {
        trace_id: trace.id.clone(),
        workflow: trace.workflow.clone(),
        source: trace.source,
        runtime_version: trace.runtime_version.clone(),
        captured_at: trace.captured_at.clone(),
        complete: trace.complete,
        function_observed: !blocks.is_empty(),
        blocks: blocks
            .into_iter()
            .map(|(start, observations)| BlockEvidence {
                start: start.to_owned(),
                observations,
            })
            .collect(),
        calls: calls
            .into_iter()
            .map(
                |((site, target, runtime_target), observations)| CallEvidence {
                    site: format!("0x{site:x}"),
                    target: target.map(|target| format!("0x{target:x}")),
                    runtime_target: runtime_target.to_owned(),
                    observations,
                },
            )
            .collect(),
        unmapped_observations: unmapped,
    }
}

fn block_containing<'a>(ranges: &BTreeMap<u64, (u64, &'a str)>, address: u64) -> Option<&'a str> {
    let (_, (end, start)) = ranges.range(..=address).next_back()?;
    (address < *end).then_some(*start)
}
