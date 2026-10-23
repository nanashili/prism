use super::measurement::Measurement;
use crate::storage::usage::{self, Usage};
use crate::ui::theme::Palette;
use eframe::egui::{self, Color32, RichText};

const COLORS: [Color32; 6] = [
    Color32::from_rgb(255, 211, 66),
    Color32::from_rgb(190, 151, 237),
    Color32::from_rgb(255, 140, 77),
    Color32::from_rgb(92, 167, 235),
    Color32::from_rgb(66, 194, 126),
    Color32::from_rgb(131, 134, 157),
];

pub(super) fn show(ui: &mut egui::Ui, measurement: &mut Measurement) {
    measurement.update(ui.ctx());
    header(ui, measurement);
    ui.add_space(12.0);
    breakdown(ui, measurement);
    location(ui);
    ui.add_space(34.0);
}

fn header(ui: &mut egui::Ui, measurement: &mut Measurement) {
    ui.horizontal(|ui| {
        let title = match &measurement.report {
            Some(report) => format!(
                "{}{} used by Prism",
                if report.incomplete.is_some() {
                    "At least "
                } else {
                    ""
                },
                usage::format_bytes(report.total()),
            ),
            None if measurement.loading() => "Measuring local storage…".into(),
            None => "Storage usage unavailable".into(),
        };
        ui.label(RichText::new(title).size(14.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let refreshing = measurement.loading();
            let response = ui
                .add_enabled_ui(!refreshing, |ui| {
                    super::button(
                        ui,
                        if refreshing {
                            "Measuring…"
                        } else {
                            "Refresh"
                        },
                    )
                })
                .inner;
            if response.clicked() {
                measurement.invalidate();
                measurement.update(ui.ctx());
            }
        });
    });
}

fn breakdown(ui: &mut egui::Ui, measurement: &Measurement) {
    let p = Palette::for_context(ui.ctx());
    if let Some(report) = &measurement.report {
        bar(ui, report);
        ui.add_space(14.0);
        legend(ui, report);
        if let Some(issue) = &report.incomplete {
            ui.add_space(12.0);
            ui.label(RichText::new(issue).size(12.0).color(p.muted));
        }
    } else {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 8.0, p.fill2);
    }
    if let Some(error) = measurement.error() {
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!("Couldn’t measure storage: {error}"))
                .size(12.0)
                .color(p.muted),
        );
    }
}

fn location(ui: &mut egui::Ui) {
    let p = Palette::for_context(ui.ctx());
    ui.add_space(14.0);
    let root = crate::settings::support_dir();
    ui.add(
        egui::Label::new(
            RichText::new(root.to_string_lossy())
                .size(12.5)
                .color(p.muted),
        )
        .wrap(),
    );
    ui.add_space(6.0);
    ui.label(RichText::new("File sizes in this folder. External projects, exports and symbolic links are excluded.").size(12.0).color(p.faint));
}

fn bar(ui: &mut egui::Ui, report: &Usage) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 8.0, Palette::for_context(ui.ctx()).fill2);
    let total = report.total();
    if total == 0 {
        return;
    }
    let first = report.bytes.iter().position(|bytes| *bytes > 0);
    let last = report.bytes.iter().rposition(|bytes| *bytes > 0);
    let mut cumulative = 0_u64;
    for (index, &bytes) in report.bytes.iter().enumerate() {
        if bytes == 0 {
            continue;
        }
        let left = rect.left() + rect.width() * (cumulative as f64 / total as f64) as f32;
        cumulative += bytes;
        let right = rect.left() + rect.width() * (cumulative as f64 / total as f64) as f32;
        let segment = egui::Rect::from_min_max(
            egui::pos2(left, rect.top()),
            egui::pos2(right, rect.bottom()),
        );
        let radius = egui::CornerRadius {
            nw: if Some(index) == first { 8 } else { 0 },
            sw: if Some(index) == first { 8 } else { 0 },
            ne: if Some(index) == last { 8 } else { 0 },
            se: if Some(index) == last { 8 } else { 0 },
        };
        ui.painter().rect_filled(segment, radius, COLORS[index]);
        ui.interact(
            segment,
            ui.id().with(("storage-segment", index)),
            egui::Sense::hover(),
        )
        .on_hover_text(format!(
            "{}: {}",
            usage::LABELS[index],
            usage::format_bytes(bytes)
        ));
    }
}

fn legend(ui: &mut egui::Ui, report: &Usage) {
    let p = Palette::for_context(ui.ctx());
    let columns = ((ui.available_width() / 160.0) as usize).clamp(1, 3);
    let width = (ui.available_width() - (columns - 1) as f32 * 12.0) / columns as f32;
    egui::Grid::new("storage-legend")
        .num_columns(columns)
        .min_col_width(width)
        .spacing([12.0, 12.0])
        .show(ui, |ui| {
            for (index, &bytes) in report.bytes.iter().enumerate() {
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 40.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.horizontal(|ui| {
                            let (dot, _) =
                                ui.allocate_exact_size(egui::vec2(8.0, 16.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot.center(), 4.0, COLORS[index]);
                            ui.label(
                                RichText::new(usage::LABELS[index])
                                    .size(12.5)
                                    .color(p.muted),
                            );
                        });
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(usage::format_bytes(bytes))
                                .size(12.5)
                                .color(p.muted),
                        );
                    },
                )
                .response
                .on_hover_text(usage::DESCRIPTIONS[index]);
                if (index + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
}
