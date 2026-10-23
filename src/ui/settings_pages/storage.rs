use super::model::{Command, Group, Path, Row};
use crate::settings::Settings;

pub(super) fn groups(s: &mut Settings) -> Vec<Group<'_>> {
    let root = crate::settings::support_dir();
    let exports = super::paths::exports_dir(s);
    let mut export = super::paths::row("Save to", &exports.to_string_lossy(), Path::Exports);
    if !s.desktop.export_dir.is_empty() {
        export = export.button("Reset", Command::ResetPath(Path::Exports));
    }
    let d = &mut s.desktop;
    vec![
        Group::new("", vec![
            Row::new("Clean up old chats", "Review chats and their saved activity before permanently deleting them.")
                .button("Review", Command::ReviewChats(false)),
        ]).foot("Deleting chats makes space reusable in the shared database. Its file may stay the same size."),
        Group::new("", vec![
            Row::new("Cached analysis", "Clear saved code views. Your projects, notes and chats are kept.")
                .button("Clear…", Command::ClearCache),
            Row::new("Archived chats", "Restore or delete individual archived chats from any project.")
                .button("Manage", Command::ReviewChats(true)),
            Row::new("Local files", "Browse generated files, agent downloads and import backups in Finder.")
                .button("Show in Finder", Command::Reveal(root)),
        ]),
        Group::new("Exports and patches", vec![
            export,
            Row::new("Show in Finder after saving", "For exported code, Objective-C lists and patched copies.")
                .toggle(&mut d.reveal_exports),
        ]),
        Group::new("More options", vec![
            Row::new("Delete all chats", "Permanently remove chats and their saved activity from every project.")
                .button("Delete all…", Command::ClearChats),
            Row::new("GPU recordings", "No recordings are created by the Rust build yet.")
                .button("Clear…", Command::ClearRecordings),
            Row::new("Save changes as you go", "Renames and notes save a moment after you make them.")
                .toggle(&mut d.autosave).blocked("Project edits are always saved locally. Disabling automatic saves is not available yet."),
            Row::new("Reset the workbench layout", "Puts panels, splits and sizes back to how they started.")
                .button("Reset", Command::ResetLayout),
        ]),
    ]
}
