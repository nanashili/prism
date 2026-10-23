use super::super::model::{Control, Path, Row};
use crate::settings::Tools;

pub(super) fn rows(tools: &mut Tools) -> (Vec<Row<'_>>, Row<'_>) {
    let together = super::profile::together_row(&mut tools.ghidra_pseudocode);
    let mut workers = Row::new(
        "Ghidra decompiler workers",
        "More workers can finish analysis sooner, but each native decompiler uses additional memory.",
    );
    workers.control = Control::Workers(&mut tools.ghidra_threads);

    let rows = vec![
        super::super::paths::row("Ghidra installation", &tools.ghidra_dir, Path::Ghidra),
        super::super::paths::row("Java home", &tools.java_home, Path::Java),
        workers,
        preparation_row(&mut tools.ghidra_analysis_time),
        Row::new(
            "Faster Ghidra analysis",
            "Skip Ghidra's basic constant reference analyzer during preparation.",
        )
        .toggle(&mut tools.fast_ghidra_analysis),
        Row::new(
            "Find parameter types",
            "Use Ghidra's decompiler parameter identification analyzer.",
        )
        .toggle(&mut tools.find_parameters),
        Row::new(
            "Rebuild switch tables",
            "Use Ghidra's decompiler switch analysis when preparing a file.",
        )
        .toggle(&mut tools.rebuild_switches),
        Row::new(
            "Find hidden code",
            "Use Ghidra's aggressive instruction finder to discover additional code.",
        )
        .toggle(&mut tools.hidden_code),
        Row::new("Read Swift names", "Use Ghidra's Swift demangler.")
            .toggle(&mut tools.swift_names),
    ];
    (rows, together)
}

fn preparation_row(time: &mut String) -> Row<'_> {
    Row::new(
        "Ghidra preparation time",
        "Maximum time to import and analyze a file before reading its functions.",
    )
    .select(
        time,
        &[
            "30 minutes",
            "1 hour",
            "2 hours",
            "4 hours",
            "6 hours",
            "8 hours",
            "12 hours",
        ],
    )
}
