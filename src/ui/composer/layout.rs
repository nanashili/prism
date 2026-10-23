use super::super::{composer_toolbar, theme::Palette};
use eframe::egui::{self, FontId, Rect, Sense};

pub(super) struct Layout {
    pub bounds: Rect,
    pub input: Rect,
    pub controls: Rect,
    pub attachments: Option<Rect>,
    pub font_size: f32,
}

pub(super) fn allocate(ui: &mut egui::Ui, draft: &str, context: bool, attachments: bool) -> Layout {
    let width = ui.available_width();
    let color = Palette::for_context(ui.ctx()).text;
    let font_size = 13.5;
    let rows = ui.fonts_mut(|fonts| {
        fonts
            .layout(
                draft.to_owned(),
                FontId::proportional(font_size),
                color,
                width - 32.0,
            )
            .rows
            .len()
    });
    let text_height = rows.clamp(1, 12) as f32 * 20.0;
    let context_height = if context { 28.0 } else { 0.0 };
    let attachment_height = if attachments { 36.0 } else { 0.0 };
    let inset = 8.0;
    let controls_height = composer_toolbar::HEIGHT;
    let height = text_height + context_height + attachment_height + controls_height + 32.0;
    let (bounds, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());

    Layout {
        bounds,
        input: Rect::from_min_size(
            bounds.min + egui::vec2(16.0, 14.0 + context_height + attachment_height),
            egui::vec2(width - 32.0, text_height),
        ),
        controls: Rect::from_min_max(
            egui::pos2(
                bounds.left() + inset,
                bounds.bottom() - inset - controls_height,
            ),
            bounds.right_bottom() - egui::vec2(inset, inset),
        ),
        attachments: attachments.then(|| {
            Rect::from_min_size(
                bounds.min + egui::vec2(14.0, 10.0 + context_height),
                egui::vec2(width - 28.0, 30.0),
            )
        }),
        font_size,
    }
}
