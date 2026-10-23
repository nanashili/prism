use super::super::model::{Control, Group, Row};
use crate::settings::{AnalysisBackend, Settings, Tools};

pub(super) fn groups(settings: &mut Settings) -> Vec<Group<'_>> {
    let backend = settings.analysis_backend();
    let mut ghidra = resource_rows(
        &mut settings.analysis_memory_mib,
        &mut settings.timeout_seconds,
        backend,
    );
    let (options, together) = analysis_rows(&mut settings.tools, backend);
    ghidra.extend(options);

    vec![
        Group::new("Together", vec![together]),
        Group::new("Ghidra", ghidra),
    ]
}

fn analysis_rows(tools: &mut Tools, _backend: AnalysisBackend) -> (Vec<Row<'_>>, Row<'_>) {
    #[cfg(feature = "prism-ghidra-rea")]
    if _backend == AnalysisBackend::JavaGhidraRea {
        return super::java_profile::rows(tools);
    }
    (Vec::new(), together_row(&mut tools.ghidra_pseudocode))
}

pub(super) fn together_row(enabled: &mut bool) -> Row<'_> {
    Row::new(
        "Use Ghidra for readable code",
        "Open files fast with Rizin, then show Ghidra's clearer pseudocode when it's ready.",
    )
    .toggle(enabled)
    .blocked("Choose an engine in the workbench. Combining Rizin and Ghidra automatically is not available yet.")
}

fn resource_rows<'a>(
    memory: &'a mut u32,
    timeout: &'a mut u64,
    backend: AnalysisBackend,
) -> Vec<Row<'a>> {
    let (memory_title, memory_description) = match backend {
        AnalysisBackend::JavaGhidraRea => (
            "Java heap budget",
            "Maximum Java heap for Ghidra. Node and native decompiler processes use additional memory.",
        ),
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

    let timeout_description = if backend == AnalysisBackend::JavaGhidraRea {
        "Minimum request allowance. Ghidra preparation time plus one minute sets a longer allowance when needed."
    } else {
        "Maximum time for each analysis request. You can also cancel an analysis at any time."
    };
    let mut timeout_row = Row::new("Request time limit", timeout_description);
    timeout_row.control = Control::Timeout(timeout);

    vec![memory_row, timeout_row]
}
