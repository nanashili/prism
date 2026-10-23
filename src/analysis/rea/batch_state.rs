use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

// REA owns envelope budgeting and cache-only retries for these saved addresses.
// A whole-envelope failure pauses the job without splitting or replaying it.
pub(super) const RUST_BATCH_SIZE: usize = 64;

pub(crate) struct DecompilationItem {
    pub address: String,
    pub result: std::result::Result<String, String>,
    pub error_details: Option<Value>,
}

pub(crate) enum DecompilationBatchPoll {
    Pending,
    Page(Vec<DecompilationItem>),
    Complete,
}

pub(crate) struct DecompilationBatch {
    pub(super) session: Arc<()>,
    pub(super) token: u64,
    pub(super) addresses: Vec<String>,
    pub(super) evidence: Option<String>,
    pub(super) offset: usize,
    pub(super) page_limit: usize,
    pub(super) expected: Counts,
    pub(super) observed: Counts,
    pub(super) stage: Stage,
}

pub(super) struct PendingCall {
    pub id: u64,
    pub started: Instant,
}

#[derive(Clone, Copy)]
pub(super) enum Request {
    Summary,
    RetainedSummary,
    Page,
    RustPage { end: usize },
}

pub(super) enum Stage {
    Waiting(PendingCall, Request),
    Ready,
    Complete,
    Failed,
}

#[derive(Default, PartialEq)]
pub(super) struct Counts {
    pub succeeded: usize,
    pub failed: usize,
}
