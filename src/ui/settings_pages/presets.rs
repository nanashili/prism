use crate::settings::permissions::Permissions;
use crate::ui::presentation::Selection;
use crate::ui::{settings_controls as controls, theme::Palette};
use eframe::egui::{self, Align2, FontId, Rect, Sense, Stroke, vec2};

const PRESETS: &[(&str, &str, &str)] = &[
    (
        "Careful",
        "Ask about everything",
        "Agents ask before reading the file or changing anything. Slowest, most control.",
    ),
    (
        "Balanced",
        "Ask before changes",
        "Agents read and move around freely, and ask before they change or run anything.",
    ),
    (
        "Free",
        "Let agents work",
        "Agents rename, take notes and run the app on their own. They still ask before patching or using your terminal.",
    ),
];

pub(in crate::ui) fn show(ui: &mut egui::Ui, permissions: &mut Permissions) {
    let width = (ui.available_width() - 24.0) / 3.0;
    let p = Palette::for_context(ui.ctx());
    let descriptions: Vec<_> = PRESETS
        .iter()
        .map(|(_, _, text)| {
            ui.painter().layout(
                (*text).into(),
                FontId::proportional(12.5),
                p.muted,
                width - 28.0,
            )
        })
        .collect();
    let height = descriptions
        .iter()
        .map(|text| text.size().y + 78.0)
        .fold(130.0, f32::max);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        for (index, (id, title, _)) in PRESETS.iter().enumerate() {
            let selected = *permissions == Permissions::preset(id);
            let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
            ui.painter().rect(
                rect,
                14.0,
                if selected {
                    p.fill
                } else {
                    egui::Color32::TRANSPARENT
                },
                Stroke::new(
                    1.5,
                    if selected {
                        egui::Color32::TRANSPARENT
                    } else {
                        p.line
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let center = rect.min + vec2(21.0, 23.0);
            controls::radio_indicator(ui, center, Selection::from(selected), &response);
            ui.painter().text(
                rect.min + vec2(36.0, 23.0),
                Align2::LEFT_CENTER,
                title,
                FontId::proportional(13.0),
                p.text,
            );
            ui.painter().galley(
                rect.min + vec2(14.0, 42.0),
                descriptions[index].clone(),
                p.muted,
            );
            if *id == "Balanced" {
                let badge =
                    Rect::from_min_size(rect.left_bottom() + vec2(14.0, -30.0), vec2(98.0, 20.0));
                ui.painter().rect_filled(badge, 10.0, p.fill2);
                ui.painter().text(
                    badge.center(),
                    Align2::CENTER_CENTER,
                    "Recommended",
                    FontId::proportional(11.5),
                    p.muted,
                );
            }
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, selected, *title)
            });
            if response.clicked() {
                *permissions = Permissions::preset(id);
            }
        }
    });
    ui.add_space(26.0);
}
