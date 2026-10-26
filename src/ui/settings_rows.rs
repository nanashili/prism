use super::{
    settings_row_layout::{Layout, Widths},
    theme::Palette,
};
use crate::ui::presentation::RowPosition;
use eframe::egui::{self, FontId, Frame, RichText};

pub(super) fn group(ui: &mut egui::Ui, label: &str, contents: impl FnOnce(&mut egui::Ui)) {
    let p = Palette::for_context(ui.ctx());
    if !label.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(label).size(12.5).color(p.muted));
        });
        ui.add_space(8.0);
    }
    Frame::NONE.fill(p.raised).corner_radius(14).show(ui, |ui| {
        ui.set_width(ui.available_width());
        contents(ui);
    });
}

pub(super) fn row(
    ui: &mut egui::Ui,
    first: RowPosition,
    title: &str,
    description: &str,
    control_width: f32,
    control: impl FnOnce(&mut egui::Ui),
) {
    let p = Palette::for_context(ui.ctx());
    let layout = Layout::allocate(
        ui,
        description,
        FontId::proportional(12.5),
        Widths {
            leading: 0.0,
            controls: control_width,
        },
    );
    layout.separator(ui, first);
    ui.painter()
        .with_clip_rect(layout.text.intersect(ui.clip_rect()))
        .text(
            layout.text.min,
            egui::Align2::LEFT_TOP,
            title,
            FontId::proportional(13.5),
            p.text,
        );
    layout.paint_description(ui);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(title)
            .max_rect(layout.controls)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 8.0;
    control(&mut child);
}
