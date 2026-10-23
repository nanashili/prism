use super::theme::Palette;
use eframe::egui::{self, FontId, Frame, Rect, RichText, Sense};

pub(super) fn group(ui: &mut egui::Ui, label: &str, contents: impl FnOnce(&mut egui::Ui)) {
    let p = Palette::for_context(ui.ctx());
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(RichText::new(label).size(12.5).color(p.muted));
    });
    ui.add_space(8.0);
    let background = if ui.visuals().dark_mode {
        egui::Color32::from_gray(28)
    } else {
        egui::Color32::from_rgb(241, 241, 239)
    };
    Frame::NONE
        .fill(background)
        .corner_radius(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            contents(ui);
        });
}

pub(super) fn row(
    ui: &mut egui::Ui,
    first: bool,
    title: &str,
    description: &str,
    control_width: f32,
    control: impl FnOnce(&mut egui::Ui),
) {
    let p = Palette::for_context(ui.ctx());
    let text_width = (ui.available_width() - control_width - 50.0).max(80.0);
    let mut description = egui::text::LayoutJob::simple(
        description.into(),
        FontId::proportional(12.5),
        p.muted,
        text_width,
    );
    for section in &mut description.sections {
        section.format.line_height = Some(18.0);
    }
    let description = ui.fonts_mut(|fonts| fonts.layout_job(description));
    let text_height = 17.0
        + if description.is_empty() {
            0.0
        } else {
            2.0 + description.size().y
        };
    let height = (text_height + 24.0).max(64.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    if !first {
        ui.painter().hline(
            rect.left() + 18.0..=rect.right() - 18.0,
            rect.top(),
            (1.0, p.line),
        );
    }
    let text_top = rect.center().y - text_height * 0.5;
    ui.painter().text(
        egui::pos2(rect.left() + 18.0, text_top),
        egui::Align2::LEFT_TOP,
        title,
        FontId::proportional(13.5),
        p.text,
    );
    if !description.is_empty() {
        ui.painter().galley(
            egui::pos2(rect.left() + 18.0, text_top + 19.0),
            description,
            p.muted,
        );
    }
    let controls = Rect::from_min_size(
        egui::pos2(rect.right() - 18.0 - control_width, rect.center().y - 15.0),
        egui::vec2(control_width, 30.0),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(title)
            .max_rect(controls)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    control(&mut child);
}
