use crate::ui::theme::Palette;
use eframe::egui::{self, Align2, FontId, Rect, Sense, Stroke};

pub(super) fn option(ui: &mut egui::Ui, number: usize, label: &str) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let width = ui.available_width();
    let text = ui.painter().layout(
        label.into(),
        FontId::proportional(13.0),
        p.text3,
        (width - 38.0).max(40.0),
    );
    let height = (text.size().y + 16.0).max(36.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 8, p.sel);
    }
    let badge = Rect::from_center_size(
        rect.left_top() + egui::vec2(11.0, 18.0),
        egui::vec2(19.0, 19.0),
    );
    ui.painter().rect(
        badge,
        5,
        p.fill,
        Stroke::new(1.0, p.line3),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        badge.center(),
        Align2::CENTER_CENTER,
        number,
        FontId::proportional(11.0),
        p.muted,
    );
    ui.painter()
        .galley(rect.min + egui::vec2(29.0, 8.0), text, p.text3);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.on_hover_text("Send this answer")
}

pub(super) fn question_icon(ui: &mut egui::Ui) {
    let p = Palette::for_context(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
    ui.painter()
        .circle_stroke(rect.center(), 5.2, Stroke::new(1.0, p.muted));
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "?",
        FontId::proportional(9.0),
        p.muted,
    );
}
