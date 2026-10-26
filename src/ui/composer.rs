use super::app::Prism;
use crate::ui::presentation::Presence;
mod completion;
mod input;
mod layout;
pub(super) mod mentions;
mod palette_items;
mod picker;
mod query;
mod usage;

pub(super) fn input_id(chat: u64) -> egui::Id {
    egui::Id::new(("composer-input", chat))
}

use super::{composer_drop, icons::Icon, theme::Palette};
use eframe::egui::{self, FontId, Rect, Stroke};

impl Prism {
    pub(super) fn composer(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        let show_context =
            self.screen == crate::model::Screen::Workbench && !self.workspace.selected.is_empty();
        let layout = layout::allocate(
            ui,
            &self.chat.draft,
            Presence::from(show_context),
            Presence::from(!self.chat.attachments.is_empty()),
        );
        let rect = layout.bounds;
        ui.painter().rect(
            rect,
            20.0,
            p.fill2,
            Stroke::new(
                1.0,
                if composer_drop::hovered(ui.ctx(), rect) {
                    p.accent
                } else {
                    p.line
                },
            ),
            egui::StrokeKind::Inside,
        );
        if show_context {
            self.composer_context(ui, rect);
        }
        if let Some(bounds) = layout.attachments {
            self.attachment_strip(ui, bounds);
        }
        let mut input_ui = ui.new_child(egui::UiBuilder::new().max_rect(layout.input));
        input_ui.set_clip_rect(input_ui.clip_rect().intersect(layout.input));
        let keyboard = self.composer_input(&mut input_ui, layout.font_size, layout.input.height());
        let mut controls_ui = ui.new_child(egui::UiBuilder::new().max_rect(layout.controls));
        let clicked = self.composer_controls(&mut controls_ui);
        self.composer_drop(ui.ctx(), rect);
        if keyboard || clicked {
            self.send_prompt();
        }
    }

    fn composer_context(&self, ui: &mut egui::Ui, composer: Rect) {
        let p = Palette::for_context(ui.ctx());
        let chip = Rect::from_min_size(
            composer.min + egui::vec2(14.0, 10.0),
            egui::vec2(composer.width() - 28.0, 24.0),
        );
        ui.painter().rect_filled(chip, 6.0, p.fill2);
        Icon::Function.paint(
            ui,
            Rect::from_center_size(
                chip.left_center() + egui::vec2(12.0, 0.0),
                egui::vec2(13.0, 13.0),
            ),
            p.accent,
        );
        let name = self
            .workspace
            .project
            .as_ref()
            .and_then(|project| project.renamed.get(&self.workspace.selected))
            .or_else(|| {
                self.workspace
                    .rows
                    .iter()
                    .find(|row| row.address == self.workspace.selected)
                    .map(|row| &row.name)
            })
            .unwrap_or(&self.workspace.selected);
        let mut label = egui::text::LayoutJob::simple_singleline(
            name.clone(),
            FontId::monospace(11.5),
            p.text3,
        );
        label.wrap.max_rows = 1;
        label.wrap.max_width = (chip.width() - 36.0).max(0.0);
        let label = ui.fonts_mut(|fonts| fonts.layout_job(label));
        ui.painter().galley(
            chip.left_center() + egui::vec2(27.0, -label.size().y / 2.0),
            label,
            p.text3,
        );
    }
}
