use super::super::{
    agent_catalog::Catalog, agent_picker, app::Prism, icons::Icon, popup,
    settings_pages::model::Command, theme::Palette, widgets,
};
use crate::ui::presentation::{Border, Caret, InstallationState};
use crate::{agent::catalog::Entry, settings::Settings};
use eframe::egui::{self, RichText};

mod cards;

#[cfg(test)]
mod tests;

impl Prism {
    pub(super) fn setup_agents(&mut self, ui: &mut egui::Ui) {
        self.agent_catalog.start(ui.ctx());
        if let Some(command) = show(ui, &mut self.agent_catalog, &mut self.settings) {
            self.settings_command(ui.ctx(), command);
        }
    }
}

fn show(ui: &mut egui::Ui, catalog: &mut Catalog, settings: &mut Settings) -> Option<Command> {
    let p = Palette::for_context(ui.ctx());
    let mut action = scan_header(ui, catalog);
    ui.add_space(10.0);

    let selected = agent_picker::id(settings).to_owned();
    if let Some(command) = detected_agents(ui, catalog, &selected) {
        action = Some(command);
    }
    if let Some(command) = current_agent(ui, catalog, settings, &selected) {
        action = Some(command);
    }

    ui.add_space(14.0);
    ui.label(
        RichText::new("Sign in through your agent before starting a chat.")
            .size(12.0)
            .color(p.muted),
    );
    ui.add_space(12.0);
    if let Some(command) = more_agents(ui, catalog, &selected) {
        action = Some(command);
    }
    if let Some(command) = custom_agent(ui, settings) {
        action = Some(command);
    }
    action
}

fn detected_agents(ui: &mut egui::Ui, catalog: &Catalog, selected: &str) -> Option<Command> {
    let detected: Vec<_> = catalog
        .entries
        .iter()
        .filter(|entry| catalog.installed.contains(&entry.id))
        .collect();
    if detected.is_empty() {
        ui.label(
            RichText::new(if catalog.scanned {
                "Install an agent, then choose Scan again. You can also pick one below or set it up later."
            } else {
                "Checking common install locations…"
            })
            .color(Palette::for_context(ui.ctx()).muted),
        );
        None
    } else {
        cards::grid(ui, &detected, selected, InstallationState::Installed)
    }
}

fn current_agent(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    settings: &Settings,
    selected: &str,
) -> Option<Command> {
    if let Some(entry) = catalog
        .entries
        .iter()
        .find(|entry| entry.id == selected && !catalog.installed.contains(&entry.id))
    {
        ui.add_space(10.0);
        return cards::grid(ui, &[entry], selected, InstallationState::Available);
    } else if !catalog.entries.iter().any(|entry| entry.id == selected) {
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!("Selected agent: {}", agent_picker::label(settings)))
                .color(Palette::for_context(ui.ctx()).text),
        );
    }
    None
}

fn scan_header(ui: &mut egui::Ui, catalog: &Catalog) -> Option<Command> {
    let p = Palette::for_context(ui.ctx());
    ui.horizontal(|ui| {
        let label = if !catalog.scanned {
            "Looking for agents…".into()
        } else if catalog.installed.is_empty() {
            "No agents found on this computer".into()
        } else {
            format!("Found on this computer · {}", catalog.installed.len())
        };
        ui.label(RichText::new(label).size(12.0).color(p.muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            widgets::chip_tinted(
                ui,
                Some(Icon::Retry),
                "Scan again",
                Border::Hidden,
                Caret::Hidden,
                p.muted,
            )
            .on_hover_text("Look for newly installed agents")
            .clicked()
            .then_some(Command::Rescan)
        })
        .inner
    })
    .inner
}

fn more_agents(ui: &mut egui::Ui, catalog: &mut Catalog, selected: &str) -> Option<Command> {
    egui::CollapsingHeader::new("Choose another agent")
        .id_salt("setup-more-agents")
        .show(ui, |ui| {
            popup::search(ui, &mut catalog.query, "Search agents".into());
            ui.add_space(8.0);
            let query = catalog.query.trim().to_lowercase();
            let available: Vec<&Entry> = catalog
                .entries
                .iter()
                .filter(|entry| !catalog.installed.contains(&entry.id))
                .filter(|entry| entry.name.to_lowercase().contains(&query))
                .collect();
            if available.is_empty() {
                ui.label("No matching agents.");
                return None;
            }
            ui.label(
                RichText::new("Additional agents may need a download when you first chat.")
                    .small()
                    .weak(),
            );
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .id_salt("setup-agent-catalog")
                .max_height(260.0)
                .show(ui, |ui| {
                    cards::grid(ui, &available, selected, InstallationState::Available)
                })
                .inner
        })
        .body_returned
        .flatten()
}

fn custom_agent(ui: &mut egui::Ui, settings: &mut Settings) -> Option<Command> {
    egui::CollapsingHeader::new("Use a custom agent")
        .id_salt("setup-custom-agent")
        .show(ui, |ui| {
            ui.label(
                RichText::new("Enter the command for an agent that supports ACP.")
                    .small()
                    .weak(),
            );
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 110.0).max(140.0);
                ui.add(
                    egui::TextEdit::singleline(&mut settings.conversation.custom_agent)
                        .hint_text("my-agent --acp")
                        .desired_width(width),
                );
                ui.add_enabled_ui(
                    !settings.conversation.custom_agent.trim().is_empty(),
                    |ui| {
                        widgets::chip(ui, None, "Use agent", Border::Hidden, Caret::Hidden)
                            .clicked()
                            .then_some(Command::CustomAgent)
                    },
                )
                .inner
            })
            .inner
        })
        .body_returned
        .flatten()
}
