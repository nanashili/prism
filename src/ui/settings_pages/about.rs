use super::model::{Command, Group, Row};
use eframe::egui;

pub(super) fn groups() -> Vec<Group<'static>> {
    let architecture = if cfg!(target_arch = "aarch64") {
        "Apple silicon"
    } else {
        "Intel"
    };
    vec![
        Group::new("", vec![
            Row::new("Prism", format!("Version {} ({architecture}). Native Rust desktop app.", env!("CARGO_PKG_VERSION")))
                .image(crate::ui::branding::LOGO)
                .button("Check for updates", Command::CheckUpdates),
            Row::new("What’s new", "Notes for every release.")
                .button("Open", Command::Open("https://github.com/nanashili/prism/releases")),
        ]),
        Group::new("Help", vec![
            Row::new("Run setup again", "Install engines and agents, and pick shortcuts.").button("Start", Command::Setup),
            Row::new("Report a problem", "Copies Prism’s version, engines and recent errors so you can paste them in a bug report. No file contents.")
                .button("Copy details", Command::CopyDetails),
            settings_file(),
        ]),
        Group::new("Made with", vec![
            Row::new("REA", "Runs Prism's Ghidra analysis and connects to optional external engines.")
                .image(egui::include_image!("../../../resources/upstream/engines/rea.svg"))
                .button("Website", Command::Open("https://github.com/morluto/rea")),
            Row::new("Rizin", "Fast reverse engineering framework.")
                .image(egui::include_image!("../../../resources/upstream/engines/rizin.svg"))
                .button("Website", Command::Open("https://rizin.re")),
            Row::new("Ghidra", "Open-source reverse engineering suite from the NSA.")
                .image(egui::include_image!("../../../resources/upstream/engines/ghidra.png"))
                .button("Website", Command::Open("https://ghidra-sre.org")),
            #[cfg(feature = "rea-rust-ghidra")]
            Row::new("Rust Ghidra", "Rust port of the Ghidra decompiler by koute, run through REA's worker.")
                .image(egui::include_image!("../../../resources/upstream/engines/ghidra.png"))
                .button("Website", Command::Open("https://github.com/koute/ghidra-decompiler")),
            Row::new("Agent Client Protocol", "How Prism talks to every agent.")
                .image(egui::include_image!("../../../resources/upstream/engines/acp-light.png"))
                .button("Website", Command::Open("https://agentclientprotocol.com")),
        ]),
    ]
}

fn settings_file() -> Row<'static> {
    let path = crate::settings::support_dir().join("settings.json");
    let mut row = Row::new("Settings file", super::paths::display(&path))
        .button("Show in Finder", Command::Reveal(path));
    row.mono = true;
    row
}
