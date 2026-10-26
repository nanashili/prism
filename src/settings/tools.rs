use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Tools {
    pub rust_ghidra_worker: String,
    pub ghidra_threads: u8,
    pub hopper_path: String,
    pub ida_path: String,
    pub stop_at_function: bool,
    pub stop_at_entry: bool,
    pub program_args: String,
    pub variable_limit: String,
    pub gpu_template: String,
    pub gpu_seconds: f32,
    pub gpu_skip_frames: String,
    pub keep_recordings: String,
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            rust_ghidra_worker: std::env::var("PRISM_RUST_GHIDRA_WORKER").unwrap_or_default(),
            ghidra_threads: 1,
            hopper_path: String::new(),
            ida_path: String::new(),
            stop_at_function: false,
            stop_at_entry: false,
            program_args: String::new(),
            variable_limit: "200".into(),
            gpu_template: "Metal System Trace".into(),
            gpu_seconds: 20.0,
            gpu_skip_frames: "3".into(),
            keep_recordings: "3".into(),
        }
    }
}

impl Tools {
    pub(super) fn same_analysis(&self, other: &Self, backend: super::AnalysisBackend) -> bool {
        if backend == super::AnalysisBackend::RustGhidraRea {
            self.rust_ghidra_worker == other.rust_ghidra_worker
                && self.ghidra_threads == other.ghidra_threads
        } else {
            true
        }
    }
}
