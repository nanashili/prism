use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Tools {
    pub ghidra_dir: String,
    pub java_home: String,
    pub rea_node: String,
    pub rust_rea: String,
    pub rust_ghidra_worker: String,
    pub ghidra_threads: u8,
    pub hopper_path: String,
    pub ida_path: String,
    pub ghidra_pseudocode: bool,
    pub ghidra_memory: String,
    pub ghidra_analysis_time: String,
    pub fast_ghidra_analysis: bool,
    pub find_parameters: bool,
    pub rebuild_switches: bool,
    pub hidden_code: bool,
    pub swift_names: bool,
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
            ghidra_dir: String::new(),
            java_home: String::new(),
            rea_node: String::new(),
            rust_rea: std::env::var("PRISM_RUST_REA").unwrap_or_default(),
            rust_ghidra_worker: std::env::var("PRISM_RUST_GHIDRA_WORKER").unwrap_or_default(),
            ghidra_threads: 1,
            hopper_path: String::new(),
            ida_path: String::new(),
            ghidra_pseudocode: false,
            ghidra_memory: "Automatic".into(),
            ghidra_analysis_time: "2 hours".into(),
            fast_ghidra_analysis: true,
            find_parameters: false,
            rebuild_switches: false,
            hidden_code: false,
            swift_names: true,
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
        match backend {
            super::AnalysisBackend::JavaGhidraRea => self.same_java_analysis(other),
            super::AnalysisBackend::RustGhidraRea => {
                self.rea_node == other.rea_node
                    && self.rust_rea == other.rust_rea
                    && self.rust_ghidra_worker == other.rust_ghidra_worker
            }
            _ => true,
        }
    }

    fn same_java_analysis(&self, other: &Self) -> bool {
        self.ghidra_dir == other.ghidra_dir
            && self.java_home == other.java_home
            && self.rea_node == other.rea_node
            && self.ghidra_threads == other.ghidra_threads
            && self.ghidra_analysis_time == other.ghidra_analysis_time
            && self.fast_ghidra_analysis == other.fast_ghidra_analysis
            && self.find_parameters == other.find_parameters
            && self.rebuild_switches == other.rebuild_switches
            && self.hidden_code == other.hidden_code
            && self.swift_names == other.swift_names
    }
}
