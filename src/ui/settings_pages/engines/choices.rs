use super::super::model::{Button, Command, Path, Row};
use crate::settings::Settings;
use eframe::egui;

pub(super) fn rows(settings: &Settings) -> Vec<Row<'static>> {
    let provider = settings.provider.as_str();
    vec![
        #[cfg(all(
            feature = "rea-rust-ghidra",
            any(target_os = "macos", target_os = "linux")
        ))]
        rea_engine(
            "Ghidra (Rust through REA)",
            "Rust Ghidra through REA. Saved-function batches reuse the prepared session and native cache. Choose the Rust REA launcher and worker below.",
            "ghidra-rust-rea",
            provider,
        ),
        #[cfg(feature = "prism-ghidra-rea")]
        rea_engine(
            "Ghidra (Java through REA)",
            "Prism's Java Ghidra provider through REA. Uses your Ghidra installation and keeps its prepared session for saved-function batches.",
            "ghidra-rea",
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

#[cfg(any(
    feature = "prism-ghidra-rea",
    all(
        feature = "rea-rust-ghidra",
        any(target_os = "macos", target_os = "linux")
    )
))]
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
    let mut row = Row::new(name, description)
        .image(image)
        .status(if found { "Installed" } else { "Not found" }, found);
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
