use super::app::Prism;
use super::{agent_option_rows, popup, widgets};
use crate::agent::options::{OptionMenu, menus};
use eframe::egui;

mod menu_cache;
mod model_selection;
mod model_selector;
mod reasoning;

pub(in crate::ui) use menu_cache::MenuCache;

impl Prism {
    pub(super) fn composer_option_menus(&self) -> std::rc::Rc<[OptionMenu]> {
        self.composer_menu_cache.get(
            self.chat_tabs.active,
            self.chat.composer_revision,
            self.agent_catalog.models_revision,
            &self.settings,
            || self.build_composer_option_menus(),
        )
    }

    fn build_composer_option_menus(&self) -> Vec<OptionMenu> {
        let mut menus = menus(&self.chat.options);
        menus.retain(|menu| {
            menu.id != "collaboration_mode" && menu.category != "collaboration_mode"
        });
        if !menus.iter().any(|menu| menu.category == "model")
            && let Some(model) = self.cached_model_menu()
        {
            menus.push(model);
        }
        let agent = crate::agent::picks::key(&self.settings);
        for menu in &mut menus {
            if menu.category == "model"
                && let Some(value) = self
                    .chat
                    .models
                    .get(&agent)
                    .filter(|value| menu.choices.iter().any(|choice| choice.value == **value))
                    .or_else(|| menu.choices.first().map(|choice| &choice.value))
            {
                menu.current.clone_from(value);
            }
            if let Some(pick) = self.chat.picks.iter().find(|pick| {
                pick.matches(menu) && menu.choices.iter().any(|choice| choice.value == pick.value)
            }) {
                menu.current.clone_from(&pick.value);
            }
        }
        menus
    }

    pub(super) fn composer_access(&mut self, ui: &mut egui::Ui, menus: &[OptionMenu]) {
        if let Some(menu) = menus.iter().find(|menu| menu.category == "mode") {
            self.access_chip(ui, menu);
        } else {
            self.permission_chip(ui);
        }
    }

    fn additional_agent_options(&mut self, ui: &mut egui::Ui, menus: &[OptionMenu]) {
        for menu in menus.iter().filter(|menu| {
            !matches!(menu.category.as_str(), "model" | "thought_level" | "mode")
                && !model_selector::is_fast_menu(menu)
        }) {
            popup::submenu(ui, super::icons::Icon::Gear, &menu.name, |ui| {
                if let Some(value) = choices(ui, menu) {
                    self.pick_option(menu, value);
                    ui.close();
                }
            });
        }
    }

    fn access_chip(&mut self, ui: &mut egui::Ui, menu: &OptionMenu) {
        let icon = agent_option_rows::mode_icon(&menu.current, selected_label(menu));
        let color = if matches!(icon, super::icons::Icon::ShieldAlert) {
            ui.visuals().warn_fg_color
        } else {
            super::theme::Palette::for_context(ui.ctx()).text3
        };
        let response =
            widgets::chip_tinted(ui, Some(icon), selected_label(menu), false, false, color);
        popup::menu(&response, 440.0)
            .align(egui::RectAlign::TOP_START)
            .show(|ui| self.access_options(ui, menu));
    }
}

fn selected_label(menu: &OptionMenu) -> &str {
    menu.choices
        .iter()
        .find(|choice| choice.value == menu.current)
        .map(|choice| choice.name.as_str())
        .unwrap_or(&menu.name)
}

fn model_label(model: Option<&OptionMenu>, effort: Option<&OptionMenu>) -> String {
    let mut label = String::new();
    for menu in [model, effort].into_iter().flatten() {
        if !label.is_empty() {
            label.push(' ');
        }
        label.push_str(selected_label(menu));
    }
    label
}

fn choices(ui: &mut egui::Ui, menu: &OptionMenu) -> Option<String> {
    let id = ui.id().with(("option-search", &menu.id));
    let mut query = ui
        .ctx()
        .data_mut(|data| data.get_temp::<String>(id))
        .unwrap_or_default();
    if menu.choices.len() > 8 {
        popup::search(ui, &mut query, format!("Search {}", menu.choices.len()));
        ui.add_space(4.0);
    }
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, query.clone()));
    let query = query.trim().to_lowercase();
    let matches: Vec<_> = menu
        .choices
        .iter()
        .filter(|choice| {
            choice.name.to_lowercase().contains(&query)
                || choice.value.to_lowercase().contains(&query)
        })
        .collect();
    let mut picked = None;
    egui::ScrollArea::vertical()
        .id_salt(&menu.id)
        .max_height(340.0)
        .show(ui, |ui| {
            for choice in matches.iter().take(60) {
                if agent_option_rows::choice(
                    ui,
                    choice,
                    &menu.category,
                    choice.value == menu.current,
                )
                .clicked()
                {
                    picked = Some(choice.value.clone());
                }
            }
            if matches.len() > 60 {
                ui.label(
                    egui::RichText::new(format!("{} more. Type to narrow.", matches.len() - 60))
                        .small()
                        .weak(),
                );
            } else if matches.is_empty() {
                ui.label("Nothing matches.");
            }
        });
    picked
}

pub(super) fn short_agent<'a>(command: &'a str, arguments: &str) -> &'a str {
    let name = command.rsplit(['/', '\\']).next().unwrap_or(command);
    if name == "codex-acp"
        || (name == "npx" && arguments.contains("@agentclientprotocol/codex-acp"))
    {
        "Codex"
    } else if name == "claude-agent-acp"
        || (name == "npx" && arguments.contains("@agentclientprotocol/claude-agent-acp"))
    {
        "Claude"
    } else if name == "gemini" {
        "Gemini"
    } else if name == "aster" {
        "Aster"
    } else if name == "opencode" {
        "OpenCode"
    } else {
        name
    }
}
