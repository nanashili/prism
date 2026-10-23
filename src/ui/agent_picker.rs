use super::app::Prism;
mod menu;
mod providers;

use super::{icons::Icon, popup};
use crate::settings::Settings;
use eframe::egui;

impl Prism {
    pub(super) fn agent_picker_content(&mut self, ui: &mut egui::Ui) {
        self.agent_catalog.start(ui.ctx());
        let selected = self
            .composer_option_menus()
            .iter()
            .find(|menu| menu.category == "model")
            .map(|menu| menu.current.clone());
        let picked = menu::show(
            ui,
            &mut self.agent_catalog,
            &self.settings,
            &self.chat,
            selected.as_deref(),
            false,
        );
        ui.separator();
        if popup::row(ui, Some(Icon::Gear), "Manage agents…", "", false).clicked() {
            self.settings_ui
                .select(super::settings_navigation::SettingsPage::Agents);
            self.screen = crate::model::Screen::Settings;
            ui.close();
        } else if let Some(picked) = picked {
            self.select_agent_model(ui.ctx(), picked);
            ui.close();
        }
    }

    fn select_agent_model(&mut self, context: &egui::Context, picked: menu::Selection) {
        self.pick_agent(context, picked.id.clone(), picked.name);
        if self.settings.agent_id != picked.id {
            return;
        }
        if let Some(value) = picked.model {
            self.apply_agent_pick(crate::agent::options::Pick {
                id: crate::agent::options::MODEL_PICK.into(),
                value,
                legacy: false,
            });
        }
        self.prepare_agent_session();
    }

    pub(super) fn pick_agent(&mut self, context: &egui::Context, id: String, name: String) {
        if self.settings.agent_id == id {
            return;
        }
        if let Err(error) = self.disconnect_chat_agent() {
            self.report_error(error);
            return;
        }
        self.settings.agent_id = id;
        self.settings.agent_name = name;
        self.persist_settings(context);
    }
}

pub(super) fn label(settings: &Settings) -> &str {
    if settings.agent_id.is_empty() {
        super::agent_options::short_agent(&settings.agent, &settings.agent_args)
    } else {
        &settings.agent_name
    }
}

pub(super) fn id(settings: &Settings) -> &str {
    if settings.agent_id.is_empty() {
        custom_id(settings)
    } else {
        &settings.agent_id
    }
}

pub(super) fn custom_id(settings: &Settings) -> &str {
    match super::agent_options::short_agent(&settings.agent, &settings.agent_args) {
        "Claude" => "claude-acp",
        "Codex" => "codex-acp",
        "Gemini" => "gemini",
        "OpenCode" => "opencode",
        "Aster" => "aster",
        other => other,
    }
}
