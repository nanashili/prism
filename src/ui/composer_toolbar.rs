use super::app::Prism;
use super::{icons::Icon, theme::Palette, widgets};
use eframe::egui::{self, Rect, Sense};

pub(super) const HEIGHT: f32 = 30.0;

impl Prism {
    pub(super) fn composer_controls(&mut self, ui: &mut egui::Ui) -> bool {
        if self.screen == crate::model::Screen::Setup {
            return self.setup_composer_controls(ui);
        }
        let clicked = self.composer_toolbar_row(ui);

        if clicked
            && self.chat.busy
            && self.chat.draft.trim().is_empty()
            && self.chat.attachments.is_empty()
        {
            self.chat.questions.clear_agent_question();
            if let Some(agent) = &self.agent {
                agent.cancel();
            }
            false
        } else {
            clicked
        }
    }

    fn composer_toolbar_row(&mut self, ui: &mut egui::Ui) -> bool {
        let menus = self.composer_option_menus();
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if widgets::icon_button(ui, Icon::New, "Add attachments", HEIGHT).clicked() {
                self.pick_attachments();
            }
            let access_width = (ui.available_width() * 0.36).clamp(64.0, 160.0);
            ui.allocate_ui_with_layout(
                egui::vec2(access_width, HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| self.composer_access(ui, &menus),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready = !self.chat.draft.trim().is_empty() || !self.chat.attachments.is_empty();
                let send = send_button(ui, ready, self.chat.busy && !ready);
                self.model_chip(ui, &menus);
                self.context_usage(ui);
                send
            })
            .inner
        })
        .inner
    }
}

fn send_button(ui: &mut egui::Ui, ready: bool, stopping: bool) -> bool {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(HEIGHT, HEIGHT), Sense::click());
    let enabled = ready || stopping;
    let fill = if enabled { p.text } else { p.faint };
    ui.painter()
        .circle_filled(rect.center(), HEIGHT * 0.5, fill);
    let icon = if stopping { Icon::Stop } else { Icon::Send };
    let color = if ui.visuals().dark_mode {
        egui::Color32::from_gray(10)
    } else {
        egui::Color32::WHITE
    };
    icon.paint(
        ui,
        Rect::from_center_size(rect.center(), egui::vec2(15.0, 15.0)),
        color,
    );
    let label = if stopping { "Stop" } else { "Send" };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response.on_hover_text(label).clicked() && enabled
}
