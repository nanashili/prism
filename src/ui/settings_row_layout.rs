use super::{presentation::RowPosition, theme::Palette};
use eframe::egui::{self, FontId, Galley, Rect, Sense, vec2};
use std::sync::Arc;

/// Shared card padding keeps labels, dividers, and controls on the same edges.
pub(super) const PADDING: f32 = 18.0;
/// Labels keep a readable column before controls move onto a separate line.
const INLINE_TEXT_WIDTH: f32 = 240.0;
const CONTROL_GAP: f32 = 14.0;
const CONTROL_LINE_HEIGHT: f32 = 42.0;

#[derive(Clone, Copy, Default)]
pub(super) struct Widths {
    pub leading: f32,
    pub controls: f32,
}

pub(super) struct Layout {
    pub bounds: Rect,
    pub text: Rect,
    pub controls: Rect,
    description: Arc<Galley>,
}

impl Layout {
    pub fn allocate(ui: &mut egui::Ui, description: &str, font: FontId, widths: Widths) -> Self {
        let content_width = (ui.available_width() - PADDING * 2.0).max(0.0);
        let inline_text_width = content_width - widths.leading - widths.controls - CONTROL_GAP;
        let stacked = widths.controls > 0.0 && inline_text_width < INLINE_TEXT_WIDTH;
        let reserved_controls = if stacked || widths.controls == 0.0 {
            0.0
        } else {
            widths.controls + CONTROL_GAP
        };
        let text_width = (content_width - widths.leading - reserved_controls).max(1.0);

        let mut job = egui::text::LayoutJob::simple(
            description.into(),
            font,
            Palette::for_context(ui.ctx()).muted,
            text_width,
        );
        for section in &mut job.sections {
            section.format.line_height = Some(18.0);
        }
        let description = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let text_height = if description.is_empty() {
            17.0
        } else {
            19.0 + description.size().y
        };
        let body_height = (text_height + 24.0).max(64.0);
        let extra_height = if stacked { CONTROL_LINE_HEIGHT } else { 0.0 };
        let (bounds, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), body_height + extra_height),
            Sense::hover(),
        );

        let text = Rect::from_min_size(
            bounds.min + vec2(PADDING + widths.leading, (body_height - text_height) * 0.5),
            vec2(text_width, text_height),
        );
        let control_center = if stacked {
            bounds.bottom() - 27.0
        } else {
            bounds.center().y
        };
        let controls = Rect::from_min_size(
            egui::pos2(
                bounds.right() - PADDING - widths.controls,
                control_center - 15.0,
            ),
            vec2(widths.controls, 30.0),
        );

        Self {
            bounds,
            text,
            controls,
            description,
        }
    }

    pub fn separator(&self, ui: &egui::Ui, position: RowPosition) {
        if position == RowPosition::Following {
            ui.painter().hline(
                self.bounds.left() + PADDING..=self.bounds.right() - PADDING,
                self.bounds.top(),
                (1.0, Palette::for_context(ui.ctx()).line),
            );
        }
    }

    pub fn paint_description(&self, ui: &egui::Ui) {
        if !self.description.is_empty() {
            ui.painter().galley(
                self.text.min + vec2(0.0, 19.0),
                Arc::clone(&self.description),
                Palette::for_context(ui.ctx()).muted,
            );
        }
    }
}
