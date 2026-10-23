use super::model::{Group, Path, Row};
use crate::settings::Settings;
use eframe::egui;

mod choices;
#[cfg(feature = "prism-ghidra-rea")]
mod java_profile;
mod profile;

pub(super) fn groups(settings: &mut Settings) -> Vec<Group<'_>> {
    let engines = choices::rows(settings);
    let tools = tool_rows(settings);
    let mut groups = profile::groups(settings);
    groups.insert(0, Group::new("Open files with", engines).foot(
        "Changing engines preserves saved results. Reopen your project to prepare the selected engine. External engines need their own installation.",
    ));
    groups.push(Group::new("Analysis tools", tools));
    groups
}

fn tool_rows(settings: &Settings) -> Vec<Row<'static>> {
    let rea_title = if cfg!(feature = "prism-ghidra-rea") {
        "REA for Java Ghidra, Hopper and IDA"
    } else {
        "REA for Hopper and IDA"
    };
    let rea = super::paths::row(rea_title, &settings.rea, Path::Rea).image(egui::include_image!(
        "../../../resources/upstream/engines/rea.svg"
    ));
    let rows = vec![
        rea,
        super::paths::row(
            "REA Node executable",
            &settings.tools.rea_node,
            Path::ReaNode,
        ),
        super::paths::row("jadx", &settings.jadx, Path::Jadx),
    ];
    #[cfg(feature = "rea-rust-ghidra")]
    let rows = {
        let mut rows = rows;
        rows.insert(
            0,
            super::paths::row(
                "REA launcher (Rust Ghidra)",
                &settings.tools.rust_rea,
                Path::RustRea,
            ),
        );
        rows.insert(
            1,
            super::paths::row(
                "REA Rust Ghidra worker",
                &settings.tools.rust_ghidra_worker,
                Path::RustGhidraWorker,
            ),
        );
        rows
    };
    rows
}
