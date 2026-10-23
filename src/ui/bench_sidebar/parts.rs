use crate::ui::{icons::Icon, theme::Palette};
use eframe::egui::{self, Align2, FontId, Rect, Response, Sense};

pub(super) fn number(count: usize) -> String {
    let digits = count.to_string();
    let mut result = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            result.push(',');
        }
        result.push(digit);
    }
    result
}

pub(super) fn heading(ui: &mut egui::Ui, title: &str, count: &str, open: &mut bool) -> Response {
    let p = Palette::for_context(ui.ctx());
    ui.add_space(4.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, p.sel);
    }
    let title = ui
        .painter()
        .layout_no_wrap(title.into(), FontId::proportional(12.0), p.faint);
    let position = rect.left_center() + egui::vec2(8.0, -title.size().y * 0.5);
    ui.painter().galley(position, title.clone(), p.faint);
    let icon = if *open {
        Icon::Disclosure
    } else {
        Icon::Forward
    };
    icon.paint(
        ui,
        Rect::from_center_size(
            egui::pos2(position.x + title.size().x + 10.0, rect.center().y),
            egui::vec2(10.0, 10.0),
        ),
        p.faint,
    );
    ui.painter().text(
        rect.right_center() - egui::vec2(8.0, 0.0),
        Align2::RIGHT_CENTER,
        count,
        FontId::proportional(12.0),
        p.faint,
    );
    if response.clicked() {
        *open = !*open;
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::CollapsingHeader,
            ui.is_enabled(),
            *open,
            title.text(),
        )
    });
    response
}

#[derive(Default)]
pub(super) struct RowStyle {
    pub icon: Option<Icon>,
    pub selected: bool,
    pub depth: usize,
    pub open: Option<bool>,
    pub compact: bool,
}

pub(super) fn row(ui: &mut egui::Ui, label: &str, count: &str, style: RowStyle) -> Response {
    let RowStyle {
        icon,
        selected,
        depth,
        open,
        compact,
    } = style;
    let p = Palette::for_context(ui.ctx());
    let height = if compact || open.is_some() {
        26.0
    } else {
        32.0
    };
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, 8.0, p.sel);
    }
    let left = rect.left() + 8.0 + depth.min(8) as f32 * 14.0;
    let leading = if let Some(open) = open {
        let icon = if open {
            Icon::Disclosure
        } else {
            Icon::Forward
        };
        icon.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(left + 3.0, rect.center().y),
                egui::vec2(10.0, 10.0),
            ),
            p.faint,
        );
        15.0
    } else {
        0.0
    };
    let icon_width = if let Some(icon) = icon {
        icon.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(left + leading + 8.5, rect.center().y),
                egui::vec2(17.0, 17.0),
            ),
            p.muted,
        );
        27.0
    } else {
        0.0
    };
    let count_galley =
        ui.painter()
            .layout_no_wrap(count.into(), FontId::proportional(12.0), p.faint);
    let available =
        (rect.right() - left - leading - icon_width - 8.0 - count_galley.size().x).max(10.0);
    let mut text =
        egui::text::LayoutJob::simple_singleline(label.into(), FontId::proportional(13.0), p.text3);
    text.wrap.max_width = available;
    text.wrap.max_rows = 1;
    text.wrap.break_anywhere = true;
    let text = ui.fonts_mut(|fonts| fonts.layout_job(text));
    ui.painter().galley(
        egui::pos2(
            left + leading + icon_width,
            rect.center().y - text.size().y * 0.5,
        ),
        text,
        p.text3,
    );
    ui.painter().galley(
        egui::pos2(
            rect.right() - 8.0 - count_galley.size().x,
            rect.center().y - count_galley.size().y * 0.5,
        ),
        count_galley,
        p.faint,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    response
}
