use super::{icons::Icon, theme::Palette};
use crate::agent::options::Choice;
use crate::ui::presentation::Selection;
use eframe::egui::{self, Color32, FontId, Rect, Sense};
use std::sync::Arc;

pub(super) struct ChoiceRow<'a> {
    pub label: &'a str,
    pub description: &'a str,
    pub icon: Option<Icon>,
    pub warning: bool,
}

pub(super) fn choice(
    ui: &mut egui::Ui,
    choice: &Choice,
    category: &str,
    selected: Selection,
) -> egui::Response {
    let selected = selected == Selection::Selected;

    ChoiceRow {
        label: &choice.name,
        description: &choice.description,
        icon: (category == "mode").then(|| mode_icon(&choice.value, &choice.name)),
        warning: false,
    }
    .show(ui, Selection::from(selected))
}

impl ChoiceRow<'_> {
    pub(super) fn show(self, ui: &mut egui::Ui, selected: Selection) -> egui::Response {
        let selected = selected == Selection::Selected;

        let p = Palette::for_context(ui.ctx());
        let color = if self.warning {
            if ui.visuals().dark_mode {
                Color32::from_rgb(255, 128, 46)
            } else {
                Color32::from_rgb(178, 65, 0)
            }
        } else {
            p.text
        };
        let detail_color = if self.warning { color } else { p.muted };
        let inset = if self.icon.is_some() { 34.0 } else { 9.0 };
        let width = (ui.available_width() - inset - 35.0).max(30.0);
        let text = ChoiceText::layout(ui, &self, width, color, detail_color);
        let height = text.title.size().y
            + text
                .detail
                .as_ref()
                .map_or(0.0, |detail| detail.size().y + 1.0)
            + 16.0;
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
        if selected || response.hovered() || response.has_focus() {
            ui.painter().rect_filled(rect, 8.0, p.sel);
        }
        if let Some(icon) = self.icon {
            icon.paint(
                ui,
                Rect::from_center_size(
                    egui::pos2(rect.left() + 15.0, rect.center().y),
                    egui::vec2(15.0, 15.0),
                ),
                color,
            );
        }
        text.paint(ui, rect.min + egui::vec2(inset, 8.0), color, detail_color);
        if selected {
            Icon::Check.paint(
                ui,
                Rect::from_center_size(
                    egui::pos2(rect.right() - 16.0, rect.center().y),
                    egui::vec2(14.0, 14.0),
                ),
                color,
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                ui.is_enabled(),
                selected,
                self.label,
            )
        });
        response
    }
}

struct ChoiceText {
    title: Arc<egui::Galley>,
    detail: Option<Arc<egui::Galley>>,
}

impl ChoiceText {
    fn layout(
        ui: &egui::Ui,
        row: &ChoiceRow<'_>,
        width: f32,
        color: Color32,
        detail_color: Color32,
    ) -> Self {
        let title = ui
            .painter()
            .layout(row.label.into(), FontId::proportional(13.5), color, width);
        let description = row.description;
        let detail = (!description.is_empty()
            && !description.starts_with("http://")
            && !description.starts_with("https://"))
        .then(|| {
            ui.painter().layout(
                description.into(),
                FontId::proportional(11.5),
                detail_color,
                width,
            )
        });
        Self { title, detail }
    }

    fn paint(self, ui: &egui::Ui, position: egui::Pos2, color: Color32, detail_color: Color32) {
        let detail_top = position.y + self.title.size().y + 1.0;
        ui.painter().galley(position, self.title, color);
        if let Some(detail) = self.detail {
            ui.painter()
                .galley(egui::pos2(position.x, detail_top), detail, detail_color);
        }
    }
}

pub(super) fn mode_icon(value: &str, name: &str) -> Icon {
    let text = format!("{value} {name}").to_ascii_lowercase();
    if text.contains("plan") {
        Icon::Plan
    } else if ["bypass", "full", "yolo", "dontask"]
        .iter()
        .any(|word| text.contains(word))
    {
        Icon::ShieldAlert
    } else if text.contains("accept") || text.contains("edit") {
        Icon::Rename
    } else if text.contains("auto") {
        Icon::Bolt
    } else {
        Icon::Ask
    }
}
