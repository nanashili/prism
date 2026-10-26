use super::model::{Command, Control, Lead, Row};
use crate::ui::presentation::{RowPosition, Selection};
use crate::ui::settings_row_layout::{self, Layout, Widths};
use crate::ui::{agent_logos, settings_controls as controls, theme::Palette};
use eframe::egui::{self, Align2, FontId, Rect, Sense, vec2};

#[derive(Clone, Copy, Default)]
pub(super) struct Columns {
    selection: f32,
    icon: f32,
}

impl Columns {
    pub fn for_rows(rows: &[Row<'_>]) -> Self {
        Self {
            selection: if rows.iter().any(|row| row.pick.is_some()) {
                32.0
            } else {
                0.0
            },
            icon: if rows.iter().any(|row| row.lead.is_some()) {
                44.0
            } else {
                0.0
            },
        }
    }
}

pub(super) fn show(
    ui: &mut egui::Ui,
    row: &mut Row<'_>,
    position: RowPosition,
    columns: Columns,
) -> Option<Command> {
    let font = if row.mono {
        FontId::monospace(12.0)
    } else {
        FontId::proportional(12.5)
    };
    let widths = Widths {
        leading: columns.selection + columns.icon,
        controls: width(ui, row),
    };
    let layout = Layout::allocate(ui, &row.description, font, widths);
    layout.separator(ui, position);
    paint_content(ui, row, &layout, columns);

    let selection_center = egui::pos2(
        layout.bounds.left() + settings_row_layout::PADDING + 9.0,
        layout.text.center().y,
    );
    let picked = paint_pick(ui, row, selection_center);
    show_controls(ui, row, &layout).or(picked)
}

fn paint_content(ui: &mut egui::Ui, row: &Row<'_>, layout: &Layout, columns: Columns) {
    let title = Rect::from_min_size(layout.text.min, vec2(layout.text.width(), 18.0));
    paint_title(ui, row, title);
    layout.paint_description(ui);
    paint_lead(
        ui,
        row,
        Rect::from_center_size(
            egui::pos2(
                layout.bounds.left() + settings_row_layout::PADDING + columns.selection + 15.0,
                layout.text.center().y,
            ),
            vec2(30.0, 30.0),
        ),
    );
}

fn show_controls(ui: &mut egui::Ui, row: &mut Row<'_>, layout: &Layout) -> Option<Command> {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(&row.title)
            .max_rect(layout.controls)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 8.0;
    let response = child.add_enabled_ui(row.unavailable.is_none(), |ui| {
        super::row_control::show(ui, row)
    });
    if let Some(reason) = row.unavailable {
        response.response.on_disabled_hover_text(reason);
        ui.interact(
            layout.bounds,
            ui.id().with((&row.title, "help")),
            Sense::hover(),
        )
        .on_hover_text(reason);
    }
    response.inner
}

fn paint_title(ui: &egui::Ui, row: &Row<'_>, rect: Rect) {
    let p = Palette::for_context(ui.ctx());
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
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

fn paint_pick(ui: &mut egui::Ui, row: &Row<'_>, center: egui::Pos2) -> Option<Command> {
    let (id, selected) = row.pick.as_ref()?;
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
    controls::radio_indicator(ui, center, Selection::from(*selected), &response);
    response.clicked().then(|| Command::Engine(id.clone()))
}
