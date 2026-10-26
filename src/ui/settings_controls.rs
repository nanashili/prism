use super::{icons::Icon, popup, theme::Palette};
use crate::ui::presentation::Selection;
use eframe::egui::{self, Align2, FontId, Rect, Sense, Stroke};

pub(super) fn button_width(ui: &egui::Ui, label: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(
            label.into(),
            FontId::proportional(12.5),
            Palette::for_context(ui.ctx()).text3,
        )
        .size()
        .x
        + 22.0
}

pub(super) fn button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    ui.add_sized(
        [button_width(ui, label), 28.0],
        egui::Button::new(egui::RichText::new(label).size(12.5).color(p.text3))
            .corner_radius(8)
            .fill(egui::Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, p.line3)),
    )
}

pub(super) fn text_input(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    width: f32,
    limit: usize,
) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    egui::Frame::NONE
        .stroke(Stroke::new(1.0, p.line3))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .font(FontId::monospace(12.0))
                    .frame(false)
                    .margin(egui::Margin::ZERO)
                    .desired_width((width - 22.0).max(0.0))
                    .char_limit(limit)
                    .hint_text(hint),
            )
        })
        .inner
}

pub(super) fn radio_indicator(
    ui: &egui::Ui,
    center: egui::Pos2,
    selected: Selection,
    response: &egui::Response,
) {
    let selected = selected == Selection::Selected;

    let p = Palette::for_context(ui.ctx());
    let color = if selected {
        p.text
    } else if response.hovered() {
        p.text3
    } else {
        p.faint
    };
    ui.painter()
        .circle_stroke(center, 7.25, Stroke::new(1.5, color));
    if selected {
        ui.painter().circle_filled(center, 3.0, color);
    }
    if response.has_focus() {
        ui.painter()
            .circle_stroke(center, 10.5, Stroke::new(1.0, p.accent));
    }
}

pub(super) fn segmented_width(ui: &egui::Ui, options: &[&str]) -> f32 {
    let font = FontId::proportional(12.5);
    4.0 + options
        .iter()
        .map(|text| {
            ui.painter()
                .layout_no_wrap((*text).into(), font.clone(), egui::Color32::WHITE)
                .size()
                .x
                + 22.0
        })
        .sum::<f32>()
}

pub(super) fn segmented(ui: &mut egui::Ui, value: &mut String, options: &[&str]) {
    let p = Palette::for_context(ui.ctx());
    let width = segmented_width(ui, options);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 30.0), Sense::hover());
    ui.painter().rect_filled(rect, 9.0, p.fill);
    let mut left = rect.left() + 2.0;
    for option in options {
        let text =
            ui.painter()
                .layout_no_wrap((*option).into(), FontId::proportional(12.5), p.text);
        let bounds = Rect::from_min_size(
            egui::pos2(left, rect.top() + 2.0),
            egui::vec2(text.size().x + 22.0, 26.0),
        );
        let selected = value == option;
        let response = ui.interact(bounds, ui.id().with(option), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::RadioButton,
                ui.is_enabled(),
                selected,
                option,
            )
        });
        if selected {
            ui.painter().rect_filled(bounds, 7.0, p.card);
        }
        ui.painter().text(
            bounds.center(),
            Align2::CENTER_CENTER,
            option,
            FontId::proportional(12.5),
            if selected && *option == "Never" {
                egui::Color32::from_rgb(224, 108, 117)
            } else if selected {
                p.text
            } else {
                p.muted
            },
        );
        if response.clicked() {
            *value = (*option).into();
        }
        left = bounds.right();
    }
}

pub(super) fn select(ui: &mut egui::Ui, value: &mut String, options: &[&str]) {
    select_sized(ui, value, options, 150.0);
}

pub(super) fn select_sized(ui: &mut egui::Ui, value: &mut String, options: &[&str], width: f32) {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), Sense::click());
    ui.painter().rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, border(ui)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        Align2::LEFT_CENTER,
        &*value,
        FontId::proportional(12.5),
        p.text,
    );
    Icon::UpDown.paint(
        ui,
        Rect::from_center_size(
            rect.right_center() - egui::vec2(13.0, 0.0),
            egui::vec2(12.0, 12.0),
        ),
        p.muted,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, ui.is_enabled(), &*value)
    });
    popup::menu(&response, width.max(200.0)).show(|ui| {
        for option in options {
            let selected = value == option;
            if popup::row(
                ui,
                None,
                option,
                if selected { "✓" } else { "" },
                Selection::from(selected),
            )
            .clicked()
            {
                *value = (*option).into();
                ui.close();
            }
        }
    });
}

pub(super) fn range(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    bounds: std::ops::RangeInclusive<f32>,
    step: f32,
    unit: &str,
) {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(212.0, 20.0), Sense::click_and_drag());
    response.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), f64::from(*value), label));
    let minimum = *bounds.start();
    let maximum = *bounds.end();
    update_range(ui, &response, rect, value, bounds, step);
    let center = rect.center().y;
    let ratio = ((*value - minimum) / (maximum - minimum)).clamp(0.0, 1.0);
    let knob = rect.left() + 7.0 + ratio * 136.0;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), center),
            egui::pos2(rect.left() + 150.0, center),
        ],
        Stroke::new(4.0, border(ui)),
    );
    ui.painter().line_segment(
        [egui::pos2(rect.left(), center), egui::pos2(knob, center)],
        Stroke::new(4.0, p.text),
    );
    ui.painter()
        .circle_filled(egui::pos2(knob, center), 7.0, p.text);
    ui.painter().text(
        rect.right_center(),
        Align2::RIGHT_CENTER,
        format!("{value:.0}{unit}"),
        FontId::monospace(12.0),
        p.text3,
    );
}

pub(super) fn toggle(ui: &mut egui::Ui, label: &str, value: &mut bool) {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 20.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *value, label)
    });
    if response.clicked() {
        *value = !*value;
    }
    ui.painter()
        .rect_filled(rect, 10.0, if *value { p.text } else { border(ui) });
    let center = egui::pos2(
        if *value {
            rect.right() - 10.0
        } else {
            rect.left() + 10.0
        },
        rect.center().y,
    );
    ui.painter()
        .circle_filled(center, 8.0, if *value { p.card } else { p.text3 });
}

fn border(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_gray(58)
    } else {
        egui::Color32::from_rgb(203, 203, 200)
    }
}

fn update_range(
    ui: &egui::Ui,
    response: &egui::Response,
    rect: Rect,
    value: &mut f32,
    bounds: std::ops::RangeInclusive<f32>,
    step: f32,
) {
    let minimum = *bounds.start();
    let maximum = *bounds.end();
    if response.clicked() {
        response.request_focus();
    }
    if ui.is_enabled()
        && (response.dragged() || response.clicked())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let ratio = ((pointer.x - rect.left() - 7.0) / 136.0).clamp(0.0, 1.0);
        *value = ((minimum + ratio * (maximum - minimum)) / step).round() * step;
    }
    if response.has_focus() && ui.is_enabled() {
        let change = ui.input_mut(|input| {
            let right = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight);
            let left = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft);
            i32::from(right) - i32::from(left)
        });
        *value = (*value + change as f32 * step).clamp(minimum, maximum);
    }
}
