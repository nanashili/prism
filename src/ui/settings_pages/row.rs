use super::model::{Command, Control, Lead, Row};
use crate::ui::{agent_logos, settings_controls as controls, theme::Palette};
use eframe::egui::{self, Align2, FontId, Rect, Sense, vec2};

pub(super) fn show(ui: &mut egui::Ui, row: &mut Row<'_>, first: bool) -> Option<Command> {
    let p = Palette::for_context(ui.ctx());
    let control_width = width(ui, row);
    let lead_width =
        if row.lead.is_some() { 44.0 } else { 0.0 } + if row.pick.is_some() { 32.0 } else { 0.0 };
    let wrap = ui.available_width() - control_width - lead_width - 50.0 < 240.0;
    let text_width =
        (ui.available_width() - 36.0 - lead_width - if wrap { 0.0 } else { control_width + 14.0 })
            .max(80.0);
    let font = if row.mono {
        FontId::monospace(12.0)
    } else {
        FontId::proportional(12.5)
    };
    let mut job = egui::text::LayoutJob::simple(row.description.clone(), font, p.muted, text_width);
    for section in &mut job.sections {
        section.format.line_height = Some(18.0);
    }
    let description = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let text_height = 17.0
        + if description.is_empty() {
            0.0
        } else {
            description.size().y + 2.0
        };
    let extra = if wrap && control_width > 0.0 {
        42.0
    } else {
        0.0
    };
    let height = (text_height + 24.0).max(64.0) + extra;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    if !first {
        ui.painter().hline(
            rect.left() + 18.0..=rect.right() - 18.0,
            rect.top(),
            (1.0, p.line),
        );
    }
    let picked = paint_pick(ui, row, rect);
    let text_top = rect.top() + (height - extra - text_height) * 0.5;
    let text_left = rect.left() + 18.0 + lead_width;
    let title_rect = Rect::from_min_size(egui::pos2(text_left, text_top), vec2(text_width, 18.0));
    paint_title(ui, row, title_rect);
    if !description.is_empty() {
        ui.painter()
            .galley(egui::pos2(text_left, text_top + 19.0), description, p.muted);
    }
    paint_lead(
        ui,
        row,
        Rect::from_center_size(
            egui::pos2(
                rect.left() + 33.0 + if row.pick.is_some() { 32.0 } else { 0.0 },
                rect.top() + (height - extra) * 0.5,
            ),
            vec2(30.0, 30.0),
        ),
    );
    let center_y = if wrap {
        rect.bottom() - 27.0
    } else {
        rect.center().y
    };
    let controls_rect = Rect::from_min_size(
        egui::pos2(rect.right() - 18.0 - control_width, center_y - 15.0),
        vec2(control_width, 30.0),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(&row.title)
            .max_rect(controls_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 8.0;
    let response = child.add_enabled_ui(row.unavailable.is_none(), |ui| {
        super::row_control::show(ui, row)
    });
    if let Some(reason) = row.unavailable {
        response.response.on_disabled_hover_text(reason);
        ui.interact(rect, ui.id().with((&row.title, "help")), Sense::hover())
            .on_hover_text(reason);
    }
    response.inner.or(picked)
}

fn paint_title(ui: &egui::Ui, row: &Row<'_>, rect: Rect) {
    let p = Palette::for_context(ui.ctx());
    let painter = ui.painter().with_clip_rect(rect);
    let text = painter.layout_no_wrap(row.title.clone(), FontId::proportional(13.5), p.text);
    painter.galley(rect.min, text.clone(), p.text);
    if let Some(tag) = row.tag {
        let left = rect.left() + text.size().x + 8.0;
        let width = text_width(ui, tag, 11.0) + 14.0;
        let bounds = Rect::from_min_size(egui::pos2(left, rect.top()), vec2(width, 18.0));
        painter.rect_filled(bounds, 9.0, p.fill2);
        painter.text(
            bounds.center(),
            Align2::CENTER_CENTER,
            tag,
            FontId::proportional(11.0),
            p.muted,
        );
    }
}

fn paint_lead(ui: &mut egui::Ui, row: &Row<'_>, rect: Rect) {
    match &row.lead {
        Some(Lead::Agent(id)) => {
            ui.painter()
                .rect_filled(rect, 8.0, Palette::for_context(ui.ctx()).fill2);
            agent_logos::paint(ui, id, rect.shrink(6.0));
        }
        Some(Lead::Image(source)) => {
            egui::Image::new(source.clone())
                .fit_to_exact_size(rect.size())
                .paint_at(ui, rect);
        }
        None => {}
    }
}

fn width(ui: &egui::Ui, row: &Row<'_>) -> f32 {
    let mut widths = Vec::new();
    if let Some((status, _)) = &row.status {
        widths.push(text_width(ui, status, 12.0) + 18.0);
    }
    let control = match &row.control {
        Control::None => 0.0,
        Control::Toggle(_) | Control::Skill(_, _) => 34.0,
        Control::Choice(_, options, true) => controls::segmented_width(ui, options),
        Control::Choice(value, _, false) => select_width(ui, value),
        Control::Memory(_) | Control::Timeout(_) => 150.0,
        #[cfg(feature = "prism-ghidra-rea")]
        Control::Workers(_) => 150.0,
        Control::Range(..) => 212.0,
        Control::Text(..) => 220.0,
        Control::Permission(..) => controls::segmented_width(ui, &["Allow", "Ask", "Never"]),
    };
    if control > 0.0 {
        widths.push(control);
    }
    for button in &row.buttons {
        widths.push(text_width(ui, &button.label, 12.5) + 22.0);
    }
    widths.iter().sum::<f32>() + 8.0 * widths.len().saturating_sub(1) as f32
}

pub(super) fn select_width(ui: &egui::Ui, value: &str) -> f32 {
    (text_width(ui, value, 12.5) + 40.0).clamp(150.0, 280.0)
}

pub(super) fn text_width(ui: &egui::Ui, text: &str, size: f32) -> f32 {
    ui.painter()
        .layout_no_wrap(
            text.into(),
            FontId::proportional(size),
            egui::Color32::WHITE,
        )
        .size()
        .x
}

fn paint_pick(ui: &mut egui::Ui, row: &Row<'_>, rect: Rect) -> Option<Command> {
    let (id, selected) = row.pick.as_ref()?;
    let center = egui::pos2(rect.left() + 27.0, rect.center().y);
    let response = ui.interact(
        Rect::from_center_size(center, vec2(28.0, 28.0)),
        ui.id().with((id, "engine")),
        Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            ui.is_enabled(),
            *selected,
            &row.title,
        )
    });
    controls::radio_indicator(ui, center, *selected, &response);
    response.clicked().then(|| Command::Engine(id.clone()))
}
