use super::{icons::Icon, theme::Palette};
use eframe::egui::{self, Align2, Color32, FontId, Rect, Response, Sense, Stroke};

pub(super) fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, size: f32) -> Response {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, p.fill);
    }
    icon.paint(
        ui,
        Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0)),
        p.muted,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response.on_hover_text(label)
}

pub(super) fn chip(
    ui: &mut egui::Ui,
    icon: Option<Icon>,
    label: &str,
    border: bool,
    caret: bool,
) -> Response {
    chip_tinted(
        ui,
        icon,
        label,
        border,
        caret,
        Palette::for_context(ui.ctx()).text3,
    )
}

pub(super) fn chip_tinted(
    ui: &mut egui::Ui,
    icon: Option<Icon>,
    label: &str,
    border: bool,
    caret: bool,
    color: Color32,
) -> Response {
    chip_contents(
        ui,
        label,
        border,
        caret,
        color,
        icon.map(|icon| move |ui: &egui::Ui, rect| icon.paint(ui, rect, color)),
    )
}

pub(super) fn chip_width(ui: &egui::Ui, label: &str, icon: bool, caret: bool) -> f32 {
    let text = ui.painter().layout_no_wrap(
        label.into(),
        FontId::proportional(12.5),
        Palette::for_context(ui.ctx()).text3,
    );
    text.size().x + chip_padding(icon, caret)
}

fn chip_padding(icon: bool, caret: bool) -> f32 {
    20.0 + if icon { 21.0 } else { 0.0 } + if caret { 17.0 } else { 0.0 }
}

fn chip_contents(
    ui: &mut egui::Ui,
    label: &str,
    border: bool,
    caret: bool,
    color: Color32,
    paint: Option<impl FnOnce(&egui::Ui, Rect)>,
) -> Response {
    let p = Palette::for_context(ui.ctx());
    let font = FontId::proportional(12.5);
    let reserve = chip_padding(paint.is_some(), caret);
    let mut job = egui::text::LayoutJob::simple_singleline(label.into(), font, color);
    job.wrap.max_width = (ui.available_width() - reserve).max(20.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let text = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let width = text.size().x + reserve;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), Sense::click());
    let fill = if response.hovered() {
        p.fill2
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        15.0,
        fill,
        if border {
            Stroke::new(1.0, p.line)
        } else {
            Stroke::NONE
        },
        egui::StrokeKind::Inside,
    );
    let mut left = rect.left() + 10.0;
    if let Some(paint) = paint {
        paint(
            ui,
            Rect::from_center_size(
                egui::pos2(left + 7.0, rect.center().y),
                egui::vec2(14.0, 14.0),
            ),
        );
        left += 21.0;
    }
    ui.painter().galley(
        egui::pos2(left, rect.center().y - text.size().y * 0.5),
        text,
        color,
    );
    if caret {
        Icon::Disclosure.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(rect.right() - 12.0, rect.center().y),
                egui::vec2(11.0, 11.0),
            ),
            p.faint,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}

pub(super) fn row(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
    height: f32,
) -> Response {
    row_with_font(
        ui,
        icon,
        label,
        selected,
        height,
        FontId::proportional(13.5),
    )
}

pub(super) fn row_with_font(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
    height: f32,
    font: FontId,
) -> Response {
    row_with_icon(ui, label, selected, height, font, |ui, rect| {
        icon.paint(ui, rect, Palette::for_context(ui.ctx()).muted);
    })
}

pub(super) fn row_with_icon(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    height: f32,
    font: FontId,
    paint: impl FnOnce(&egui::Ui, Rect),
) -> Response {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, 8.0, p.sel);
    }
    paint(
        ui,
        Rect::from_center_size(
            egui::pos2(rect.left() + 16.5, rect.center().y),
            egui::vec2(17.0, 17.0),
        ),
    );
    let text_rect = Rect::from_min_max(
        egui::pos2(rect.left() + 35.0, rect.top()),
        rect.right_bottom(),
    );
    let mut job = egui::text::LayoutJob::simple_singleline(label.into(), font, p.text3);
    job.wrap.max_width = text_rect.width();
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let text = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.painter().galley(
        text_rect.left_center() - egui::vec2(0.0, text.size().y * 0.5),
        text,
        p.text3,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::SelectableLabel, ui.is_enabled(), label)
    });
    response
}

pub(super) fn section(ui: &mut egui::Ui, title: &str) {
    let p = Palette::for_context(ui.ctx());
    ui.add_space(14.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), Sense::hover());
    ui.painter().text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(12.0),
        p.faint,
    );
}
