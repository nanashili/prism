use super::app::Prism;
use super::{icons::Icon, popup, settings_navigation::SettingsPage, theme::Palette, widgets};
use crate::model::Screen;
use crate::settings::permissions::Level;
use eframe::egui::{self, Align2, FontId, Rect, Sense};

impl Prism {
    pub(super) fn permission_chip(&mut self, ui: &mut egui::Ui) {
        let level = self.settings.permissions.shell;
        let (word, description) = run_code(level);
        let response = ui
            .add_enabled_ui(!self.chat.busy, |ui| {
                widgets::chip(ui, Some(Icon::Ask), word, false, true).on_hover_text(description)
            })
            .inner;
        popup::menu(&response, 240.0)
            .align(egui::RectAlign::TOP_START)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                popup::heading(ui, "Running code");
                for choice in Level::ALL {
                    if run_code_row(ui, choice, choice == level).clicked() {
                        self.settings.permissions.shell = choice;
                        self.persist_settings(ui.ctx());
                        ui.close();
                    }
                }
                ui.add_space(4.0);
                if popup::row_with_height(ui, None, "All permissions…", "", false, 32.0).clicked()
                {
                    self.settings_ui.select(SettingsPage::Permissions);
                    self.screen = Screen::Settings;
                    ui.close();
                }
            });
    }
}

pub(super) fn run_code(level: Level) -> (&'static str, &'static str) {
    match level {
        Level::Allow => ("Auto", "Runs code without asking"),
        Level::Ask => ("Ask", "Asks before running code"),
        Level::Never => ("Blocked", "Can't run code"),
    }
}

fn run_code_row(ui: &mut egui::Ui, level: Level, selected: bool) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), Sense::click());
    if selected || response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 7.0, p.sel);
    }
    let (word, description) = run_code(level);
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, -7.0),
        Align2::LEFT_CENTER,
        word,
        FontId::proportional(13.0),
        p.text,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, 8.0),
        Align2::LEFT_CENTER,
        description,
        FontId::proportional(11.5),
        p.muted,
    );
    if selected {
        Icon::Check.paint(
            ui,
            Rect::from_center_size(
                rect.right_center() - egui::vec2(16.5, 0.0),
                egui::vec2(13.0, 13.0),
            ),
            p.text,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            ui.is_enabled(),
            selected,
            word,
        )
    });
    response
}
