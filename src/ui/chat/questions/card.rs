use super::rows;
use crate::ui::{icons::Icon, theme::Palette, widgets};
use eframe::egui::{self, Frame, RichText, Stroke};
use prism::mcp::question::{Answer, MAX_ANSWER_CHARS, Question};

pub(super) fn show(ui: &mut egui::Ui, question: &Question, draft: &mut String) -> Option<Answer> {
    let p = Palette::for_context(ui.ctx());
    let mut action = None;
    Frame::NONE
        .fill(p.fill2)
        .stroke(Stroke::new(1.0, p.line3))
        .corner_radius(18)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            action = header(ui, &question.title);
            ui.add_space(10.0);
            egui::ScrollArea::vertical()
                .id_salt("question-options")
                .max_height((ui.ctx().content_rect().height() * 0.4).clamp(120.0, 320.0))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(&question.question).size(13.0)).wrap());
                    ui.add_space(10.0);
                    for (index, option) in question.options.iter().enumerate() {
                        if rows::option(ui, index + 1, option).clicked() {
                            action = Some(Answer::Answered {
                                answer: option.clone(),
                                selected_option: Some(index + 1),
                            });
                        }
                    }
                });
            ui.add_space(12.0);
            if let Some(answer) = response_bar(ui, draft) {
                action = Some(answer);
            }
        });
    action
}

fn header(ui: &mut egui::Ui, title: &str) -> Option<Answer> {
    let p = Palette::for_context(ui.ctx());
    let mut action = None;
    ui.horizontal(|ui| {
        rows::question_icon(ui);
        ui.add(egui::Label::new(RichText::new(title).size(12.0).color(p.muted)).truncate());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::icon_button(ui, Icon::Close, "Dismiss question", 22.0).clicked() {
                action = Some(Answer::Dismissed);
            }
        });
    });
    action
}

fn response_bar(ui: &mut egui::Ui, draft: &mut String) -> Option<Answer> {
    let p = Palette::for_context(ui.ctx());
    let mut action = None;
    ui.horizontal(|ui| {
        Icon::Rename.show(ui, 14.0, p.muted);
        let width = (ui.available_width() - 114.0).max(40.0);
        let input = ui.add_sized(
            [width, 28.0],
            egui::TextEdit::singleline(draft)
                .id_salt("question-response")
                .frame(false)
                .char_limit(MAX_ANSWER_CHARS)
                .hint_text(RichText::new("Or write your own response").color(p.faint)),
        );
        let entered = (input.has_focus() || input.lost_focus())
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ready = !draft.trim().is_empty();
            let send = ui.add_enabled(
                ready,
                egui::Button::new(RichText::new("Send").color(p.card))
                    .corner_radius(16)
                    .fill(p.text),
            );
            if ready && (send.clicked() || entered) {
                action = Some(Answer::Answered {
                    answer: draft.trim().into(),
                    selected_option: None,
                });
            }
            if ui
                .add(
                    egui::Button::new("Skip")
                        .corner_radius(16)
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, p.line3)),
                )
                .clicked()
            {
                action = Some(Answer::Skipped);
            }
        });
    });
    action
}
