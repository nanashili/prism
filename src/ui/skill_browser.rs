use super::{
    app::Prism, settings_controls as controls, settings_navigation::SettingsPage, settings_pages,
    settings_rows as rows, theme::Palette,
};
use crate::ui::presentation::RowPosition;
use eframe::egui::{self, RichText};

impl Prism {
    pub(super) fn skill_browser(&mut self, ui: &mut egui::Ui, query: &str) {
        self.skill_browser_controls(ui);
        ui.add_space(26.0);

        let needle = if query.is_empty() {
            self.settings_ui.libraries.query.trim().to_lowercase()
        } else {
            query.to_owned()
        };
        let entries: Vec<_> = self
            .playbooks
            .all(&self.settings)
            .into_iter()
            .filter(|entry| self.settings_ui.libraries.show_all || entry.relevant(&self.settings))
            .filter(|entry| {
                format!("{} {}", entry.title, entry.description)
                    .to_lowercase()
                    .contains(&needle)
            })
            .collect();
        self.skill_browser_status(ui, entries.len());

        let groups = settings_pages::skill_groups(&mut self.settings, &entries);
        let (action, _) = settings_pages::show(
            ui,
            groups,
            SettingsPage::Skills,
            "",
            &mut self.settings_ui.confirm,
        );
        if let Some(action) = action {
            self.settings_command(ui.ctx(), action);
        }
    }

    fn skill_browser_controls(&mut self, ui: &mut egui::Ui) {
        rows::group(ui, "Preferences", |ui| {
            let search_width = 220.0 + 8.0 + controls::button_width(ui, "Refresh");
            rows::row(
                ui,
                RowPosition::First,
                "Search skills",
                "Filter by name or description. Refresh after editing skill files.",
                search_width,
                |ui| {
                    controls::text_input(
                        ui,
                        &mut self.settings_ui.libraries.query,
                        "Search skills",
                        220.0,
                        200,
                    );
                    if controls::button(ui, "Refresh").clicked() {
                        self.playbooks.invalidate();
                    }
                },
            );
            rows::row(
                ui,
                RowPosition::Following,
                "Show skills in the composer",
                "Type / in the composer to pick a skill.",
                34.0,
                |ui| {
                    controls::toggle(
                        ui,
                        "Show skills in the composer",
                        &mut self.settings.desktop.skills_in_composer,
                    );
                },
            );
            rows::row(
                ui,
                RowPosition::Following,
                "Show all categories and archives",
                "Include skills hidden by your Workspace categories or library preferences.",
                34.0,
                |ui| {
                    controls::toggle(
                        ui,
                        "Show all categories and archives",
                        &mut self.settings_ui.libraries.show_all,
                    );
                },
            );
        });
    }

    fn skill_browser_status(&self, ui: &mut egui::Ui, count: usize) {
        let p = Palette::for_context(ui.ctx());
        ui.label(
            RichText::new(format!("{count} matching skills"))
                .size(12.0)
                .color(p.muted),
        );
        if count == 0 && !self.playbooks.loading() {
            ui.label(
                RichText::new("Try another search or show skills from all categories.")
                    .size(12.5)
                    .color(p.muted),
            );
        }
        if self.playbooks.loading() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Indexing skills…").size(12.0).color(p.muted));
            });
        }
        for warning in &self.playbooks.warnings {
            ui.colored_label(ui.visuals().warn_fg_color, warning);
        }
        ui.add_space(12.0);
    }
}
