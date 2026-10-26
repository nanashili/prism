use crate::settings::{research::Category, skill_library::SkillLibrary};
use crate::ui::{
    app::Prism,
    playbooks, popup,
    presentation::{RowPosition, Selection},
    settings_controls as controls, settings_pages, settings_rows as rows,
    theme::Palette,
};
use eframe::egui::{self, RichText};
use std::sync::atomic::Ordering;

enum Action {
    Download,
    Import,
    Cancel,
    UseSuggested,
}

impl Prism {
    pub(in crate::ui) fn skill_settings(&mut self, ui: &mut egui::Ui, query: &str) {
        self.skill_browser(ui, query);
        self.skill_library_controls(ui);
        self.installed_skill_libraries(ui);
    }

    fn skill_library_controls(&mut self, ui: &mut egui::Ui) {
        let busy = self.settings_ui.libraries.pending.is_some();
        let mut action = None;
        rows::group(ui, "Add a library", |ui| {
            rows::row(
                ui,
                RowPosition::First,
                "GitHub repository",
                "A public repository containing skill Markdown and license files. Requires Git.",
                300.0,
                |ui| {
                    controls::text_input(
                        ui,
                        &mut self.settings_ui.libraries.repository,
                        "https://github.com/owner/repository",
                        300.0,
                        2048,
                    );
                },
            );
            let width = controls::button_width(ui, "Download")
                + 8.0
                + controls::button_width(ui, "Import folder…");
            rows::row(
                ui,
                RowPosition::Following,
                "Add skills",
                "Download from GitHub or connect a local skills folder.",
                width,
                |ui| {
                    let can_download =
                        !busy && !self.settings_ui.libraries.repository.trim().is_empty();
                    if ui
                        .add_enabled_ui(can_download, |ui| controls::button(ui, "Download"))
                        .inner
                        .clicked()
                    {
                        action = Some(Action::Download);
                    }
                    if ui
                        .add_enabled_ui(!busy, |ui| controls::button(ui, "Import folder…"))
                        .inner
                        .clicked()
                    {
                        action = Some(Action::Import);
                    }
                },
            );
            if suggested_library(ui) {
                action = Some(Action::UseSuggested);
            }
            if busy && library_progress(ui) {
                action = Some(Action::Cancel);
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new("Libraries stay outside the app bundle. Adding one does not run its scripts or start an MCP server. Source archives are not downloaded.")
                .size(12.0)
                .color(Palette::for_context(ui.ctx()).muted),
        );

        match action {
            Some(Action::Download) => self.download_skill_library(ui.ctx()),
            Some(Action::Import) => self.import_skill_folder(ui.ctx()),
            Some(Action::Cancel) => self
                .settings_ui
                .libraries
                .cancelled
                .store(true, Ordering::Relaxed),
            Some(Action::UseSuggested) => {
                self.settings_ui.libraries.repository = playbooks::download::GAME_LIBRARY.into();
            }
            None => {}
        }
    }

    fn installed_skill_libraries(&mut self, ui: &mut egui::Ui) {
        let busy = self.settings_ui.libraries.pending.is_some();
        let mut removal = None;
        for library in &mut self.settings.desktop.skill_libraries {
            ui.add_space(26.0);
            if library_settings(ui, library, busy) {
                removal = Some(library.clone());
            }
        }

        if let Some(library) = removal {
            self.remove_skill_library(ui.ctx(), library);
        }
    }
}

fn suggested_library(ui: &mut egui::Ui) -> bool {
    let width = controls::button_width(ui, "Use repository")
        + 8.0
        + controls::button_width(ui, "View repository");
    let mut use_suggested = false;
    rows::row(
        ui,
        RowPosition::Following,
        "Suggested library",
        "mcp-gamehacking",
        width,
        |ui| {
            use_suggested = controls::button(ui, "Use repository").clicked();
            if controls::button(ui, "View repository").clicked() {
                ui.ctx()
                    .open_url(egui::OpenUrl::new_tab(playbooks::download::GAME_LIBRARY));
            }
        },
    );
    use_suggested
}

fn library_progress(ui: &mut egui::Ui) -> bool {
    let mut cancel = false;
    rows::row(
        ui,
        RowPosition::Following,
        "Updating library…",
        "Skills will refresh when the operation finishes.",
        controls::button_width(ui, "Cancel") + 24.0,
        |ui| {
            ui.add(egui::Spinner::new().size(16.0));
            cancel = controls::button(ui, "Cancel").clicked();
        },
    );
    cancel
}

fn library_settings(ui: &mut egui::Ui, library: &mut SkillLibrary, busy: bool) -> bool {
    let title = library
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut description = settings_pages::paths::display(&library.path);
    if let Some(revision) = &library.revision {
        description.push_str(&format!(" · Revision {revision}"));
    }
    let remove_label = if library.repository.is_some() {
        "Remove download"
    } else {
        "Disconnect"
    };
    let mut remove = false;
    ui.push_id(library.path.clone(), |ui| {
        rows::group(ui, &title, |ui| {
            let width = 34.0 + 8.0 + controls::button_width(ui, remove_label);
            rows::row(
                ui,
                RowPosition::First,
                "Library enabled",
                &description,
                width,
                |ui| {
                    controls::toggle(ui, "Library enabled", &mut library.enabled);
                    remove = ui
                        .add_enabled_ui(!busy, |ui| controls::button(ui, remove_label))
                        .inner
                        .clicked();
                },
            );
            rows::row(
                ui,
                RowPosition::Following,
                "Include archived projects",
                "Show skill summaries from archived projects in this library.",
                34.0,
                |ui| {
                    controls::toggle(
                        ui,
                        "Include archived projects",
                        &mut library.include_archives,
                    );
                },
            );
            library_categories(ui, &mut library.fallback_categories);
        });
    });
    remove
}

fn library_categories(ui: &mut egui::Ui, categories: &mut Vec<Category>) {
    let description = if categories.is_empty() {
        "No categories selected.".into()
    } else {
        categories
            .iter()
            .map(|category| category.label())
            .collect::<Vec<_>>()
            .join(", ")
    };
    rows::row(
        ui,
        RowPosition::Following,
        "Categories without topic metadata",
        &description,
        controls::button_width(ui, "Choose…"),
        |ui| {
            let response = controls::button(ui, "Choose…");
            popup::menu(&response, 280.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    for category in Category::ALL {
                        let selected = categories.contains(&category);
                        if popup::row(
                            ui,
                            None,
                            category.label(),
                            if selected { "✓" } else { "" },
                            Selection::from(selected),
                        )
                        .clicked()
                        {
                            if selected {
                                categories.retain(|entry| *entry != category);
                            } else {
                                categories.push(category);
                            }
                        }
                    }
                });
        },
    );
}
