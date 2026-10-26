use super::app::Prism;
use super::{icons::Icon, theme::Palette};
use crate::ui::presentation::Interaction;
use eframe::egui::{self, Align2, FontId, Rect, Sense, Stroke};

impl Prism {
    pub(super) fn home(&mut self, ui: &mut egui::Ui) {
        if !self.chat.messages.is_empty() || self.chat.questions.is_pending() {
            self.home_conversation(ui);
            return;
        }
        let width = (ui.available_width() - 48.0).clamp(280.0, 700.0);
        let height = 420.0;
        let left = ui.max_rect().center().x - width * 0.5;
        let top =
            ui.max_rect().top() + 32.0 + ((ui.available_height() - 96.0 - height) * 0.5).max(0.0);
        let rect = Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, height));
        let mut interacting = false;
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            let p = Palette::for_context(ui.ctx());
            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Image::new(super::branding::LOGO)
                        .fit_to_exact_size(egui::vec2(56.0, 56.0)),
                );
                ui.add_space(14.0);
                ui.add_sized(
                    [width, 34.0],
                    egui::Label::new(
                        egui::RichText::new(self.landing.content().title)
                            .size(28.0)
                            .color(p.text),
                    ),
                );
            });
            ui.add_space(22.0);
            self.composer(ui);
            ui.add_space(16.0);
            if self.settings.desktop.show_suggestions {
                interacting = self.landing_suggestions(ui, width);
            }
            if self.settings.desktop.show_whats_new {
                ui.add_space(18.0);
                if ui.link("What’s new in Prism").clicked() {
                    self.news = Some(0);
                }
            }
        });

        self.landing.update(
            ui.ctx(),
            Interaction::from(
                interacting
                    || !self.chat.draft.is_empty()
                    || !self.chat.messages.is_empty()
                    || self.chat.busy
                    || self.palette
                    || self.error.is_some()
                    || self.notifications.open
                    || self.news.is_some()
                    || self.chat_rename.is_some(),
            ),
        );
    }

    fn landing_suggestions(&mut self, ui: &mut egui::Ui, width: f32) -> bool {
        let categories: Vec<_> = crate::settings::research::Category::ALL
            .into_iter()
            .filter(|category| self.settings.research.contains(*category))
            .take(4)
            .collect();
        let content = self.landing.content();
        let mut interacting = false;
        egui::Grid::new("suggestions")
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                for (index, category) in categories.into_iter().enumerate() {
                    let text = match category {
                        crate::settings::research::Category::Binaries => content.prompts[0],
                        crate::settings::research::Category::Mobile => content.prompts[1],
                        crate::settings::research::Category::Graphics => content.prompts[3],
                        _ => category.prompt(),
                    };
                    let kind = category.label();
                    let icon = match category {
                        crate::settings::research::Category::Graphics => Icon::Gpu,
                        crate::settings::research::Category::Binaries => Icon::Binary,
                        _ => Icon::App,
                    };
                    let response = suggestion(ui, icon, kind, text, (width - 10.0) * 0.5);
                    interacting |= response.hovered() || response.has_focus();
                    if response.clicked() {
                        self.chat.draft = text.into();
                        self.send_prompt();
                    }
                    if index % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
        interacting
    }
}

fn suggestion(ui: &mut egui::Ui, icon: Icon, kind: &str, text: &str, width: f32) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 68.0), Sense::click());
    let opacity = if response.hovered() { 0.72 } else { 1.0 };
    ui.painter().rect_stroke(
        rect,
        14.0,
        Stroke::new(1.0, p.fill2),
        egui::StrokeKind::Inside,
    );
    let point = rect.min + egui::vec2(14.0, 12.0);
    icon.paint(
        ui,
        Rect::from_min_size(point + egui::vec2(0.0, 3.0), egui::vec2(14.0, 14.0)),
        p.muted.gamma_multiply(opacity),
    );
    ui.painter().text(
        point + egui::vec2(20.0, 10.0),
        Align2::LEFT_CENTER,
        kind,
        FontId::proportional(11.5),
        p.muted.gamma_multiply(opacity),
    );
    ui.painter().with_clip_rect(rect.shrink(12.0)).text(
        point + egui::vec2(0.0, 33.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.5),
        p.text.gamma_multiply(opacity),
    );
    response
}
