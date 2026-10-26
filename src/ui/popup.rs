use super::{icons::Icon, theme::Palette};
use crate::ui::presentation::Selection;
use eframe::egui::{self, Align2, FontId, Frame, Rect, Response, Sense, Stroke};

pub(super) fn menu(response: &Response, width: f32) -> egui::Popup<'_> {
    egui::Popup::menu(response)
        .gap(6.0)
        .width(width)
        .frame(surface(&response.ctx, 6))
}

pub(super) fn surface(context: &egui::Context, horizontal_padding: i8) -> Frame {
    let p = Palette::for_context(context);
    Frame::NONE
        .fill(p.raised)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(horizontal_padding, 6))
        .shadow(egui::epaint::Shadow {
            offset: [0, 16],
            blur: 40,
            spread: 0,
            color: egui::Color32::from_black_alpha(89),
        })
}

pub(super) fn submenu(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    content: impl FnOnce(&mut egui::Ui),
) {
    let response = row(ui, Some(icon), label, "›", Selection::Unselected);
    egui::containers::menu::SubMenu::new().show(ui, &response, |ui| {
        ui.set_width(220.0);
        content(ui);
    });
}

pub(super) fn row(
    ui: &mut egui::Ui,
    icon: Option<Icon>,
    label: &str,
    hint: &str,
    selected: Selection,
) -> Response {
    let selected = selected == Selection::Selected;

    row_with_height(ui, icon, label, hint, Selection::from(selected), 30.0)
}

pub(super) fn row_with_height(
    ui: &mut egui::Ui,
    icon: Option<Icon>,
    label: &str,
    hint: &str,
    selected: Selection,
    height: f32,
) -> Response {
    let selected = selected == Selection::Selected;

    let p = Palette::for_context(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    if selected || response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(rect, if height > 30.0 { 8.0 } else { 6.0 }, p.sel);
    }
    let color = if ui.is_enabled() { p.text } else { p.faint };
    let mut left = rect.left() + 8.0;
    if let Some(icon) = icon {
        icon.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(left + 7.0, rect.center().y),
                egui::vec2(14.0, 14.0),
            ),
            if ui.is_enabled() { p.text3 } else { p.faint },
        );
        left += 22.0;
    }
    let hint = ui
        .painter()
        .layout_no_wrap(hint.into(), FontId::proportional(11.0), p.faint);
    let mut text =
        egui::text::LayoutJob::simple_singleline(label.into(), FontId::proportional(13.5), color);
    text.wrap.max_width = (rect.right() - left - hint.size().x - 20.0).max(0.0);
    text.wrap.max_rows = 1;
    let text = ui.fonts_mut(|fonts| fonts.layout_job(text));
    ui.painter().galley(
        egui::pos2(left, rect.center().y - text.size().y / 2.0),
        text,
        color,
    );
    ui.painter().galley(
        egui::pos2(
            rect.right() - 8.0 - hint.size().x,
            rect.center().y - hint.size().y / 2.0,
        ),
        hint,
        p.faint,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}

pub(super) fn heading(ui: &mut egui::Ui, title: &str) {
    let p = Palette::for_context(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), Sense::hover());
    ui.painter().text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(11.5),
        p.muted,
    );
}

pub(super) fn search(ui: &mut egui::Ui, query: &mut String, hint: String) -> Response {
    let p = Palette::for_context(ui.ctx());
    Frame::NONE
        .fill(p.fill)
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(8, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                Icon::Search.show(ui, 13.0, p.muted);
                ui.add(
                    egui::TextEdit::singleline(query)
                        .char_limit(200)
                        .min_size(egui::vec2(0.0, 18.0))
                        .frame(false)
                        .margin(0.0)
                        .hint_text(hint)
                        .font(FontId::proportional(12.5))
                        .desired_width(ui.available_width()),
                )
            })
            .inner
        })
        .inner
}
