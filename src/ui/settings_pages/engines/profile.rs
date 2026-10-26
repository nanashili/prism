use super::super::model::{Control, Group, Row};
use crate::settings::{AnalysisBackend, Settings};

pub(super) fn groups(settings: &mut Settings) -> Vec<Group<'_>> {
    let backend = settings.analysis_backend();
    let resources = resource_rows(
        &mut settings.analysis_memory_mib,
        &mut settings.timeout_seconds,
        backend,
    );
    vec![Group::new("Ghidra", resources)]
}

fn resource_rows<'a>(
    memory: &'a mut u32,
    timeout: &'a mut u64,
    backend: AnalysisBackend,
) -> Vec<Row<'a>> {
    let (memory_title, memory_description) = match backend {
        AnalysisBackend::RustGhidraRea => (
            "Worker memory threshold",
            "REA samples its Rust Ghidra worker's resident memory and stops it above this threshold. Memory can rise between samples; other REA processes use additional memory.",
        ),
        _ => (
            "Analysis memory budget",
            "Memory setting for the selected analysis engine. Enforcement depends on that engine.",
        ),
    };
    let mut memory_row = Row::new(memory_title, memory_description);
    memory_row.control = Control::Memory(memory);

    let timeout_description =
        "Maximum time for each analysis request. You can also cancel an analysis at any time.";
    let mut timeout_row = Row::new("Request time limit", timeout_description);
    timeout_row.control = Control::Timeout(timeout);

    vec![memory_row, timeout_row]
}
