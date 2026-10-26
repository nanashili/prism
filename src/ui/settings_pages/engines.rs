use super::model::{Group, Path, Row};
use crate::settings::Settings;
use eframe::egui;

mod choices;
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
    vec![
        super::paths::row(
            "REA launcher (nanashili/rea-rust)",
            &settings.rea,
            Path::Rea,
        )
        .image(egui::include_image!(
            "../../../resources/upstream/engines/rea.svg"
        )),
        #[cfg(feature = "rea-rust-ghidra")]
        super::paths::row(
            "REA worker override (advanced)",
            &settings.tools.rust_ghidra_worker,
            Path::RustGhidraWorker,
        ),
        super::paths::row("jadx", &settings.jadx, Path::Jadx),
    ]
}
