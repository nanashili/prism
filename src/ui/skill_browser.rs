use super::{app::Prism, playbooks::Playbook};
use crate::settings::Settings;
use eframe::egui::{self, RichText};

impl Prism {
    pub(super) fn skill_browser(&mut self, ui: &mut egui::Ui, query: &str) {
        self.skill_browser_controls(ui);
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
        ui.label(format!(
            "{} matching skills. Category choices are in Settings > Workspace.",
            entries.len()
        ));
        if self.playbooks.loading() {
            ui.label("Indexing skills…");
        }
        for warning in &self.playbooks.warnings {
            ui.colored_label(ui.visuals().warn_fg_color, warning);
        }
        egui::ScrollArea::vertical()
            .id_salt("skill-browser")
            .max_height(420.0)
            .show_rows(ui, 64.0, entries.len(), |ui, range| {
                for entry in &entries[range] {
                    skill_row(ui, entry, &mut self.settings);
                }
            });
    }

    fn skill_browser_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Skills").strong());
            if ui.button("Refresh").clicked() {
                self.playbooks.invalidate();
            }
            if ui.button("New skill…").clicked()
                && let Err(error) = self.create_skill()
            {
                self.error = Some(error.to_string());
            }
            if ui.button("Choose custom skills folder…").clicked() {
                self.settings_command(
                    ui.ctx(),
                    super::settings_pages::model::Command::Choose(
                        super::settings_pages::model::Path::Skills,
                    ),
                );
            }
        });
        ui.checkbox(
            &mut self.settings.desktop.skills_in_composer,
            "Show skills in the composer",
        );
        ui.checkbox(
            &mut self.settings_ui.libraries.show_all,
            "Show skills from all categories and archives",
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.settings_ui.libraries.query)
                .hint_text("Search skills")
                .desired_width(ui.available_width())
                .char_limit(200),
        );
    }
}

fn skill_row(ui: &mut egui::Ui, entry: &Playbook, settings: &mut Settings) {
    ui.push_id(&entry.id, |ui| {
        let mut enabled = !entry.disabled(&settings.desktop.disabled_skills);
        if ui.checkbox(&mut enabled, entry.title.as_ref()).changed() {
            let key = entry
                .setting_key(&settings.desktop.disabled_skills)
                .to_owned();
            settings.desktop.disabled_skills.retain(|id| *id != key);
            if !enabled {
                settings.desktop.disabled_skills.push(key);
            }
        }
        ui.add(
            egui::Label::new(RichText::new(entry.description.as_ref()).small().weak()).truncate(),
        )
        .on_hover_text(entry.description.as_ref());
        let categories = entry
            .categories
            .iter()
            .map(|category| category.label())
            .collect::<Vec<_>>()
            .join(", ");
        let status = if entry.relevant(settings) {
            ""
        } else {
            "Hidden by workspace or library choices. "
        };
        ui.add(
            egui::Label::new(
                RichText::new(format!("{status}{categories}"))
                    .small()
                    .weak(),
            )
            .truncate(),
        );
    });
}
