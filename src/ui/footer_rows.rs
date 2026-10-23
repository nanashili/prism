use super::{agent_logos, icons::Icon, theme::Palette};
use eframe::egui::{self, Align2, FontId, Rect, Response, Sense};

pub(super) enum Lead<'a> {
    Icon(Icon),
    Agent(&'a str),
    Engine(&'a str),
    Prism,
}

pub(super) enum Tail<'a> {
    None,
    Hint(&'a str),
    Submenu(&'a str),
    Check(bool),
}

pub(super) fn row(
    ui: &mut egui::Ui,
    lead: Lead<'_>,
    label: &str,
    note: &str,
    tail: Tail<'_>,
) -> Response {
    let p = Palette::for_context(ui.ctx());
    let height = if note.is_empty() { 32.0 } else { 42.0 };
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    if response.hovered() || response.has_focus() || matches!(tail, Tail::Check(true)) {
        ui.painter().rect_filled(rect, 8.0, p.sel);
    }
    let icon = Rect::from_center_size(
        rect.left_center() + egui::vec2(16.5, 0.0),
        egui::vec2(17.0, 17.0),
    );
    paint_lead(ui, lead, icon);
    let text_right = paint_tail(ui, rect, &tail);
    let color = if ui.is_enabled() { p.text3 } else { p.faint };
    let painter = ui.painter().with_clip_rect(Rect::from_min_max(
        egui::pos2(rect.left() + 35.0, rect.top()),
        egui::pos2(text_right - 8.0, rect.bottom()),
    ));
    painter.text(
        egui::pos2(
            rect.left() + 35.0,
            rect.center().y - if note.is_empty() { 0.0 } else { 7.5 },
        ),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.5),
        color,
    );
    if !note.is_empty() {
        painter.text(
            egui::pos2(rect.left() + 35.0, rect.center().y + 8.0),
            Align2::LEFT_CENTER,
            note,
            FontId::proportional(11.5),
            p.faint,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}

fn paint_tail(ui: &egui::Ui, rect: Rect, tail: &Tail<'_>) -> f32 {
    let p = Palette::for_context(ui.ctx());
    let (hint, arrow, check) = match tail {
        Tail::None => ("", false, false),
        Tail::Hint(hint) => (*hint, false, false),
        Tail::Submenu(hint) => (*hint, true, false),
        Tail::Check(checked) => ("", false, *checked),
    };
    let right = rect.right() - 8.0;
    if arrow || check {
        let size = if check { 15.0 } else { 11.0 };
        let icon = if check { Icon::Check } else { Icon::Forward };
        icon.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(right - 7.5, rect.center().y),
                egui::vec2(size, size),
            ),
            p.faint,
        );
    }
    let right = right - if arrow || check { 24.0 } else { 0.0 };
    let text = ui
        .painter()
        .layout_no_wrap(hint.into(), FontId::proportional(12.0), p.faint);
    let left = right - text.size().x;
    ui.painter().galley(
        egui::pos2(left, rect.center().y - text.size().y / 2.0),
        text,
        p.faint,
    );
    left
}

pub(super) fn paint_lead(ui: &egui::Ui, lead: Lead<'_>, rect: Rect) {
    match lead {
        Lead::Icon(icon) => icon.paint(ui, rect, Palette::for_context(ui.ctx()).text3),
        Lead::Agent(id) => agent_logos::paint(ui, id, rect),
        Lead::Engine(provider) => {
            let image = match provider {
                "ghidra" | "ghidra-rea" | "ghidra-rust-rea" => {
                    egui::include_image!("../../resources/upstream/engines/ghidra.png")
                }
                "ida" => egui::include_image!("../../resources/upstream/engines/ida.png"),
                "hopper" => egui::include_image!("../../resources/upstream/engines/hopper.png"),
                _ => egui::include_image!("../../resources/upstream/engines/rizin.svg"),
            };
            egui::Image::new(image).paint_at(ui, rect);
        }
        Lead::Prism => {
            egui::Image::new(super::branding::LOGO).paint_at(ui, rect);
        }
    }
}
