use super::{model_label, reasoning, selected_label};
use crate::agent::options::{Choice, OptionMenu};
use crate::ui::icons::Icon;
use crate::ui::{agent_option_rows::ChoiceRow, app::Prism, popup, settings_controls, widgets};
use eframe::egui;

impl Prism {
    pub(in crate::ui) fn model_chip(&mut self, ui: &mut egui::Ui, menus: &[OptionMenu]) {
        let model = menus.iter().find(|menu| menu.category == "model");
        let effort = menus.iter().find(|menu| menu.category == "thought_level");
        let label = if model.is_some() || effort.is_some() {
            model_label(model, effort)
        } else {
            "Select model".into()
        };
        let usage_width = self
            .chat
            .usage
            .as_ref()
            .filter(|usage| usage.size > 0)
            .map_or(0.0, |_| 24.0 + ui.spacing().item_spacing.x);
        let width = widgets::chip_width(ui, &label, false, true)
            .min((ui.available_width() - usage_width).max(0.0));
        let response = ui
            .allocate_ui_with_layout(
                egui::vec2(width, super::super::composer_toolbar::HEIGHT),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| widgets::chip(ui, None, &label, false, true),
            )
            .inner;
        let view = response.id.with("model-selector-page");
        if response.clicked() {
            let page = if model.is_none() && effort.is_none() {
                Page::Agents
            } else {
                Page::Reasoning
            };
            ui.ctx().data_mut(|data| data.insert_temp(view, page));
        }
        let page = ui
            .ctx()
            .data(|data| data.get_temp::<Page>(view))
            .unwrap_or_default();
        popup::menu(&response, if page == Page::Agents { 300.0 } else { 256.0 })
            .align(egui::RectAlign::TOP_END)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| self.model_selector_content(ui, menus, view, page));
    }

    fn model_selector_content(
        &mut self,
        ui: &mut egui::Ui,
        menus: &[OptionMenu],
        view: egui::Id,
        page: Page,
    ) {
        let model = menus.iter().find(|menu| menu.category == "model");
        let effort = menus.iter().find(|menu| menu.category == "thought_level");
        if page == Page::Agents || (model.is_none() && effort.is_none()) {
            self.agent_picker_content(ui);
            return;
        }

        if let Some(effort) = effort.filter(|_| page == Page::Reasoning) {
            match reasoning::show(ui, effort, model.map(selected_label), fast_menu(menus)) {
                Some(reasoning::Action::Models) => {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(view, Page::Models));
                }
                Some(reasoning::Action::Pick(menu, value)) => self.pick_option(menu, value),
                None => {}
            }
        } else if let Some(model) = model {
            if effort.is_none()
                && let Some(value) = reasoning::model_header(ui, fast_menu(menus))
                && let Some(fast) = fast_menu(menus)
            {
                self.pick_option(fast, value);
            }
            if let Some(value) = model_list(ui, model) {
                self.pick_option(model, value);
                ui.close();
            }
        }

        ui.add_space(4.0);
        ui.separator();
        if popup::row(ui, Some(Icon::App), "Agents and models…", "", false).clicked() {
            ui.ctx()
                .data_mut(|data| data.insert_temp(view, Page::Agents));
        }
        self.additional_agent_options(ui, menus);
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
enum Page {
    #[default]
    Reasoning,
    Models,
    Agents,
}

fn fast_menu(menus: &[OptionMenu]) -> Option<&OptionMenu> {
    menus.iter().find(|menu| is_fast_menu(menu))
}

pub(super) fn is_fast_menu(menu: &OptionMenu) -> bool {
    matches!(menu.id.as_str(), "fast-mode" | "fast_mode")
        && ["on", "off"]
            .iter()
            .all(|value| menu.choices.iter().any(|choice| choice.value == *value))
}

fn model_list(ui: &mut egui::Ui, menu: &OptionMenu) -> Option<String> {
    let filter_id = ui.id().with(("daybreak", &menu.id));
    let mut daybreak = ui.ctx().data_mut(|data| {
        *data.get_temp_mut_or_insert_with(filter_id, || {
            menu.choices
                .iter()
                .any(|choice| choice.value == menu.current && is_daybreak(choice))
        })
    });
    if menu.choices.iter().any(is_daybreak) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Daybreak").size(12.5));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                settings_controls::toggle(ui, "Show Daybreak models", &mut daybreak);
            });
        });
        ui.ctx()
            .data_mut(|data| data.insert_temp(filter_id, daybreak));
        ui.add_space(6.0);
        ui.separator();
    }
    popup::heading(ui, "Select model");
    let query = model_search(ui, menu);
    let mut picked = None;
    egui::ScrollArea::vertical()
        .id_salt(("models", &menu.id))
        .max_height(340.0)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            let mut shown = 0;
            for choice in &menu.choices {
                if (!daybreak && is_daybreak(choice))
                    || (!choice.name.to_lowercase().contains(&query)
                        && !choice.value.to_lowercase().contains(&query))
                {
                    continue;
                }
                shown += 1;
                if model_row(ui, choice, choice.value == menu.current).clicked() {
                    picked = Some(choice.value.clone());
                }
            }
            if shown == 0 {
                ui.label(egui::RichText::new("No models match.").small().weak());
            }
        });
    picked
}

fn model_search(ui: &mut egui::Ui, menu: &OptionMenu) -> String {
    if menu.choices.len() <= 12 {
        return String::new();
    }
    let id = ui.id().with(("model-search", &menu.id));
    let mut query = ui
        .ctx()
        .data_mut(|data| data.get_temp::<String>(id).unwrap_or_default());
    popup::search(ui, &mut query, "Search models".into());
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, query.clone()));
    query.trim().to_lowercase()
}

fn model_row(ui: &mut egui::Ui, choice: &Choice, selected: bool) -> egui::Response {
    if choice.value.eq_ignore_ascii_case("default") || choice.name.eq_ignore_ascii_case("default") {
        return ChoiceRow {
            label: &choice.name,
            description: &choice.description,
            icon: None,
            warning: false,
        }
        .show(ui, selected);
    }
    popup::row(
        ui,
        None,
        &choice.name,
        if selected { "✓" } else { "" },
        selected,
    )
    .on_hover_text(&choice.description)
}

fn is_daybreak(choice: &Choice) -> bool {
    choice.value.to_ascii_lowercase().contains("daybreak")
        || choice.name.to_ascii_lowercase().contains("daybreak")
}
