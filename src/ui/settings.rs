use super::app::Prism;
use super::{
    settings_navigation::{SETTINGS_GROUPS, SettingsPage},
    settings_pages, theme,
};
use crate::worker::Job;
use eframe::egui::{self, RichText};

impl Prism {
    pub(super) fn settings_screen(&mut self, ui: &mut egui::Ui) {
        let before = self.settings.clone();
        let query = self.settings_ui.query.trim().to_lowercase();
        let page = self.settings_ui.page;
        if page == SettingsPage::Agents || !query.is_empty() {
            self.agent_catalog.start(ui.ctx());
        }
        let section = if page == SettingsPage::Storage && query.is_empty() {
            self.settings_ui.storage.title()
        } else {
            page.title()
        };
        egui::ScrollArea::vertical()
            .id_salt(("settings", section, &query))
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = (ui.available_width() - 56.0).clamp(200.0, 720.0);
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        ui.max_rect().center().x - width * 0.5,
                        ui.cursor().top() + 40.0,
                    ),
                    egui::vec2(width, ui.available_height()),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    self.settings_content(ui, page, &query);
                });
            });
        if self.settings != before {
            if self.settings.tools.program_args != before.tools.program_args {
                self.debug.args = self.settings.tools.program_args.clone();
            }
            self.queue_settings_save(ui.ctx());
        }
    }

    fn settings_content(&mut self, ui: &mut egui::Ui, page: SettingsPage, query: &str) {
        let p = theme::Palette::for_context(ui.ctx());
        let storage = page == SettingsPage::Storage && query.is_empty();
        let title = if storage {
            self.settings_ui.storage.title()
        } else if query.is_empty() {
            page.title()
        } else {
            "Results"
        };
        let intro = if storage {
            self.settings_ui.storage.intro().to_owned()
        } else if query.is_empty() {
            page.intro().to_owned()
        } else {
            format!("Settings that match “{}”.", self.settings_ui.query.trim())
        };
        ui.label(RichText::new(title).size(20.0).strong());
        ui.add_space(2.0);
        ui.label(RichText::new(intro).size(13.0).color(p.muted));
        ui.add_space(20.0);

        if query.is_empty() {
            self.settings_page(ui, page, "");
        } else {
            self.settings_results(ui, query);
        }
        ui.add_space(56.0);
    }

    fn settings_results(&mut self, ui: &mut egui::Ui, query: &str) {
        let playbooks = self.playbooks.all(&self.settings);
        let mut found = false;
        for &page in SETTINGS_GROUPS.iter().flat_map(|group| group.iter()) {
            if super::settings_search::page_matches(
                page,
                query,
                &mut self.settings,
                &self.agent_catalog,
                &playbooks,
            ) {
                found = true;
                self.settings_page(ui, page, query);
            }
        }
        if !found {
            let p = theme::Palette::for_context(ui.ctx());
            let message = format!(
                "Nothing matches “{}”. Try a word like font, folder, Ghidra or rename.",
                self.settings_ui.query.trim()
            );
            ui.label(RichText::new(message).size(13.0).color(p.muted));
        }
    }

    pub(super) fn settings_page(&mut self, ui: &mut egui::Ui, page: SettingsPage, query: &str) {
        match page {
            SettingsPage::Skills => {
                self.skill_settings(ui, query);
                return;
            }
            SettingsPage::Workspace => {
                self.workspace_categories(ui, query);
                return;
            }
            SettingsPage::Appearance => {
                super::appearance::show(ui, &mut self.settings);
                return;
            }
            SettingsPage::Keyboard => {
                super::keyboard::show(
                    ui,
                    &mut self.settings,
                    &mut self.settings_ui.keyboard,
                    query,
                );
                return;
            }
            SettingsPage::Permissions if query.is_empty() => {
                settings_pages::presets(ui, &mut self.settings.permissions)
            }
            SettingsPage::Storage if query.is_empty() => {
                if self.storage_settings(ui) {
                    return;
                }
            }
            _ => {}
        }
        let playbooks = if page == SettingsPage::Skills {
            self.playbooks.all(&self.settings)
        } else {
            Vec::new()
        };
        let groups = settings_pages::groups(
            page,
            &mut self.settings,
            settings_pages::PageContext {
                agents: &self.agent_catalog,
                playbooks: &playbooks,
                browsing_agents: self.settings_ui.browsing_agents,
            },
            &mut self.settings_ui.custom_tool_server,
        );
        let (action, destination) =
            settings_pages::show(ui, groups, page, query, &mut self.settings_ui.confirm);
        if let Some(destination) = destination {
            self.settings_ui.select(destination);
        }
        if let Some(action) = action {
            self.settings_command(ui.ctx(), action);
        }
    }

    pub(super) fn persist_settings(&mut self, context: &egui::Context) {
        self.settings_ui.pending_save = None;
        match self.settings.save() {
            Ok(()) => {
                theme::apply(context, &self.settings);
                self.enqueue(Job::Configure(Box::new(self.settings.clone())));
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(super) fn queue_settings_save(&mut self, context: &egui::Context) {
        theme::apply(context, &self.settings);
        self.settings_ui.pending_save = Some(std::time::Instant::now());
        context.request_repaint_after(std::time::Duration::from_millis(350));
    }

    pub(super) fn flush_settings(&mut self, context: &egui::Context) {
        let Some(changed) = self.settings_ui.pending_save else {
            return;
        };
        let delay = std::time::Duration::from_millis(350);
        if changed.elapsed() >= delay {
            self.persist_settings(context);
        } else {
            context.request_repaint_after(delay.saturating_sub(changed.elapsed()));
        }
    }
}
