use super::model::{Command, Group, Row};
use crate::settings::Settings;
use crate::ui::presentation::StatusTone;

const UPDATER: &str = "Automatic installation is not available in the Rust build. Check for updates opens the release page.";

pub(super) fn groups(s: &mut Settings) -> Vec<Group<'_>> {
    let d = &mut s.desktop;
    vec![
        Group::new("Startup", vec![
            Row::new("When Prism opens", "Start on Home, or go straight back to what you had open.")
                .select(&mut d.launch, &["Home", "The last project"]),
            Row::new("Reopen tabs and splits", "Bring back the workbench layout of each project when you open it.")
                .toggle(&mut d.restore_layout).blocked("Saving and restoring tab layouts is not available yet."),
        ]),
        Group::new("Updates", vec![
            Row::new("Update automatically", "Download updates in the background and install them when you quit Prism.")
                .toggle(&mut d.auto_update).blocked(UPDATER),
            Row::new("Update channel", "Beta gets new features a few weeks early.")
                .segmented(&mut d.update_channel, &["Stable", "Beta"]).blocked(UPDATER),
            Row::new(concat!("Prism ", env!("CARGO_PKG_VERSION")), "Check the latest published release.")
                .button("Check now", Command::CheckUpdates),
        ]),
        Group::new("Home", vec![
            Row::new("Show what’s new on Home", "A short note on Home after each update. Dismiss it and it stays gone.")
                .toggle(&mut d.show_whats_new),
            Row::new("Show suggested questions", "The starter questions under the composer on Home.")
                .toggle(&mut d.show_suggestions),
        ]),
        Group::new("Reset", vec![
            Row::new("Reset all settings", "Puts every setting back the way it was on day one. Your projects and chats stay.")
                .button("Reset…", Command::Reset),
        ]),
    ]
}

pub(super) fn privacy(s: &mut Settings) -> Vec<Group<'_>> {
    let agent = crate::ui::agent_picker::label(s).to_owned();
    let d = &mut s.desktop;
    vec![
        Group::new("What leaves your computer", vec![
            Row::new("The agent you use", "Code and questions you send go to that agent’s provider, under its own privacy terms. Local agents keep everything here.")
                .status(agent, StatusTone::Neutral),
            Row::new("Hide your name in paths", "Agents see ~/Downloads instead of /Users/yourname/Downloads.")
                .toggle(&mut d.hide_paths).blocked("Path redaction across analysis and agent tool responses is not available yet."),
            Row::new("Ask before sending a whole file", "Ask when the agent requests the entire open file through its approval system. The agent’s own file permissions also apply.")
                .toggle(&mut d.ask_before_whole_file),
        ]).foot("Projects, chats and caches stay on this Mac unless you export them or share them with an agent."),
        Group::new("Help improve Prism", vec![
            Row::new("Count this install", "Sends a random ID, the version and your system when Prism opens, so we know how many people use it.")
                .toggle(&mut d.count_install).blocked("The Rust build does not send telemetry."),
            Row::new("Send crash reports", "Where Prism crashed, its version and your system. Never your files or chats.")
                .toggle(&mut d.crash_reports).blocked("The Rust build does not send crash reports."),
            Row::new("Share how features are used", "Counts like how often the graph is opened. Off by default.")
                .toggle(&mut d.usage_stats).blocked("The Rust build does not collect usage statistics."),
        ]),
    ]
}
