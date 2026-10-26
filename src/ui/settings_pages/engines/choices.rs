use super::super::model::{Button, Command, Path, Row};
use crate::settings::Settings;
use crate::ui::presentation::StatusTone;
use eframe::egui;

pub(super) fn rows(settings: &Settings) -> Vec<Row<'static>> {
    let provider = settings.provider.as_str();
    vec![
        #[cfg(feature = "rea-rust-ghidra")]
        rea_engine(
            "Ghidra (Rust through REA)",
            "Native nanashili/rea-rust Ghidra. Saved-function batches reuse the prepared session and native cache. Choose the launcher in Analysis tools below.",
            "ghidra-rust-rea",
            provider,
        ),
        engine(
            "Rizin",
            "Fast, opens most apps in seconds.",
            &settings.rizin,
            Path::Rizin,
            egui::include_image!("../../../../resources/upstream/engines/rizin.svg"),
            provider == "rizin",
        ),
        engine(
            "Hopper",
            "Uses your Hopper license through its own tools.",
            &settings.tools.hopper_path,
            Path::Hopper,
            egui::include_image!("../../../../resources/upstream/engines/hopper.png"),
            provider == "hopper",
        ),
        engine(
            "IDA Pro",
            "Uses your IDA license. Needs IDA 9 with its MCP plugin.",
            &settings.tools.ida_path,
            Path::Ida,
            egui::include_image!("../../../../resources/upstream/engines/ida.png"),
            provider == "ida",
        ),
    ]
}

#[cfg(feature = "rea-rust-ghidra")]
fn rea_engine(name: &str, description: &str, id: &str, selected: &str) -> Row<'static> {
    let mut row = Row::new(name, description).image(egui::include_image!(
        "../../../../resources/upstream/engines/ghidra.png"
    ));
    row.pick = Some((id.into(), selected == id));
    if selected == id {
        row.tag = Some("Opens your files");
    }
    row
}

fn engine(
    name: &str,
    description: &str,
    path: &str,
    key: Path,
    image: egui::ImageSource<'static>,
    selected: bool,
) -> Row<'static> {
    let found = !path.is_empty() && std::path::Path::new(path).exists();
    let mut row = Row::new(name, description).image(image).status(
        if found { "Installed" } else { "Not found" },
        StatusTone::from(found),
    );
    let id = match key {
        Path::Rizin => "rizin",
        Path::Hopper => "hopper",
        _ => "ida",
    };
    if selected {
        row.tag = Some("Opens your files");
    }
    if found {
        row.pick = Some((id.into(), selected));
        if matches!(key, Path::Rizin) {
            row = row.button(
                "Check for update",
                Command::Open("https://github.com/rizinorg/rizin/releases"),
            );
        }
        let mut remove = Button::new("Remove", Command::ResetPath(key));
        remove.unavailable = Some(
            "Removing an installed engine from disk is not available here. Manage the installation with Finder or your package manager.",
        );
        row.buttons.push(remove);
        row
    } else {
        let row = row.button("Locate…", Command::Choose(key));
        if matches!(key, Path::Hopper | Path::Ida) {
            row.blocked("Configure this engine in REA before starting Prism. Custom application paths are not passed to REA yet.")
        } else {
            row
        }
    }
}
