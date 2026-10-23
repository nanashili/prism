use super::{settings_controls, settings_rows};
use crate::settings::permissions::{Level, Permissions, Tool};
use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, permissions: &mut Permissions) {
    settings_rows::group(ui, "Permission preset", |ui| {
        settings_rows::row(
            ui,
            true,
            "How much to ask",
            "Choose a starting point, then adjust each permission below.",
            222.0,
            |ui| {
                let mut preset = ["Careful", "Balanced", "Free"]
                    .into_iter()
                    .find(|name| *permissions == Permissions::preset(name))
                    .unwrap_or("Custom")
                    .to_owned();
                let before = preset.clone();
                settings_controls::segmented(ui, &mut preset, &["Careful", "Balanced", "Free"]);
                if preset != before {
                    *permissions = Permissions::preset(&preset);
                }
            },
        );
    });
    ui.add_space(20.0);
    for (group, rows) in GROUPS {
        settings_rows::group(ui, group, |ui| {
            for (index, (tool, title, description)) in rows.iter().enumerate() {
                settings_rows::row(ui, index == 0, title, description, 185.0, |ui| {
                    let mut label = permissions.level(*tool).label().to_owned();
                    settings_controls::segmented(ui, &mut label, &["Allow", "Ask", "Never"]);
                    if let Some(level) = Level::from_label(&label) {
                        permissions.set(*tool, level);
                    }
                });
            }
        });
        ui.add_space(20.0);
    }
    ui.label(egui::RichText::new(
        "Applies to approval requests sent by your ACP agent on the next turn. The agent’s own modes and tool permissions also apply."
    ).small().weak());
}

type PermissionRow = (Tool, &'static str, &'static str);

const GROUPS: &[(&str, &[PermissionRow])] = &[
    (
        "Reading",
        &[
            (
                Tool::Read,
                "Read the open file",
                "Functions, strings, code and bytes in the app you have open.",
            ),
            (
                Tool::View,
                "Move around the window",
                "Open functions, switch views and jump to addresses for you.",
            ),
        ],
    ),
    (
        "Changing your project",
        &[
            (
                Tool::Rename,
                "Rename functions and variables",
                "Saved in the project. You can undo a rename.",
            ),
            (
                Tool::Notes,
                "Add notes, comments and bookmarks",
                "Saved in the project next to the code.",
            ),
            (
                Tool::Patch,
                "Patch bytes",
                "Writes a patched copy. The original file is never touched.",
            ),
        ],
    ),
    (
        "Running things",
        &[
            (
                Tool::Gpu,
                "Run the app to record a GPU trace",
                "Applies to GPU profiling requests. GPU capture is not available yet.",
            ),
            (
                Tool::Debug,
                "Run the app in the debugger",
                "Applies to agent debugger approval requests.",
            ),
            (
                Tool::Shell,
                "Run commands on your computer",
                "The agent’s own terminal tools, outside Prism.",
            ),
        ],
    ),
];
