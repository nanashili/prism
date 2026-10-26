//! Bounded, relocatable observations from a single workflow and target module.

use super::graph::address_value;
use crate::error::{Result, failure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Per-trace input limit; persistence also caps the number of retained traces.
pub(crate) const MAX_TRACE_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_EVENTS: usize = 10_000;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecutionTrace {
    pub version: u32,
    pub id: String,
    pub workflow: String,
    pub source: TraceSource,
    pub runtime_version: String,
    pub captured_at: String,
    pub target_sha256: String,
    pub mapping: ModuleMapping,
    /// Collector-reported interval completion; this never establishes workflow coverage.
    pub complete: bool,
    pub events: Vec<Observation>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TraceSource {
    Debugger,
    FridaStalker,
    Imported,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModuleMapping {
    pub runtime_base: String,
    pub static_base: String,
    pub size: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Observation {
    pub kind: ObservationKind,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub thread_id: u64,
    pub count: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationKind {
    Instruction,
    Block,
    Function,
    Call,
    /// A stopped program counter is a sample, not a function-entry hit or call edge.
    Sample,
}

impl ExecutionTrace {
    /// Candidate entries must still match an indexed function exactly.
    pub(crate) fn executed_addresses(&self) -> BTreeMap<u64, u64> {
        let mut addresses = BTreeMap::new();
        for event in &self.events {
            if event.kind != ObservationKind::Sample
                && let Some(address) = self.mapping.static_address(&event.address)
            {
                *addresses.entry(address).or_default() += event.count;
            }
            if let Some(target) = event.target.as_deref()
                && let Some(address) = self.mapping.static_address(target)
            {
                *addresses.entry(address).or_default() += event.count;
            }
        }
        addresses
    }

    pub(crate) fn validate(&self, target: &str) -> Result<()> {
        if self.version != 1 || self.target_sha256 != target {
            return Err(failure(
                "Trace version or target SHA-256 does not match the open project",
            ));
        }
        if target.len() != 64 || !target.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(failure("A native target SHA-256 is required"));
        }
        for (name, value, limit) in [
            ("id", self.id.as_str(), 128),
            ("workflow", self.workflow.as_str(), 1024),
            ("runtime_version", self.runtime_version.as_str(), 256),
            ("captured_at", self.captured_at.as_str(), 128),
        ] {
            if value.trim().is_empty() || value.len() > limit {
                return Err(failure(format!("Invalid trace {name}")));
            }
        }

        self.mapping.validate()?;
        if self.events.is_empty() || self.events.len() > MAX_EVENTS {
            return Err(failure(
                "A trace must contain between 1 and 10000 observations",
            ));
        }
        for observation in &self.events {
            observation.validate()?;
        }

        if serde_json::to_vec(self)?.len() > MAX_TRACE_BYTES {
            return Err(failure("Trace exceeds the 2 MiB limit"));
        }
        Ok(())
    }
}

impl ModuleMapping {
    fn validate(&self) -> Result<()> {
        let runtime = parse_address(&self.runtime_base)?;
        let static_base = parse_address(&self.static_base)?;
        if self.size == 0
            || runtime.checked_add(self.size).is_none()
            || static_base.checked_add(self.size).is_none()
        {
            return Err(failure("Invalid or overflowing module address range"));
        }
        Ok(())
    }

    /// External-module addresses remain unresolved instead of being guessed.
    pub(crate) fn static_address(&self, address: &str) -> Option<u64> {
        let offset = address_value(address)?.checked_sub(address_value(&self.runtime_base)?)?;
        if offset >= self.size {
            return None;
        }
        address_value(&self.static_base)?.checked_add(offset)
    }
}

impl Observation {
    fn validate(&self) -> Result<()> {
        parse_address(&self.address)?;
        if let Some(target) = &self.target {
            parse_address(target)?;
        }
        if (self.kind == ObservationKind::Call) != self.target.is_some() {
            return Err(failure("Only call observations require a target address"));
        }
        if self.count == 0 || self.count > 1_000_000_000 {
            return Err(failure(
                "Observation counts must be between 1 and 1000000000",
            ));
        }
        Ok(())
    }
}

fn parse_address(address: &str) -> Result<u64> {
    let digits = address.strip_prefix("0x").unwrap_or(address);
    if digits.is_empty()
        || digits.len() > 16
        || !digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(failure(format!("Invalid trace address: {address}")));
    }
    address_value(address).ok_or_else(|| failure(format!("Invalid trace address: {address}")))
}
