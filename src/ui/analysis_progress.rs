use super::app::Prism;
use super::{icons::Icon, theme::Palette, widgets};
use crate::events::AnalysisProgress;
use eframe::egui::{self, Align, Layout, Rect, Sense};
use std::time::Duration;

impl Prism {
    pub(super) fn analysis_progress(&self, ui: &mut egui::Ui) {
        let Some(progress) = &self.analysis else {
            return;
        };
        let p = Palette::for_context(ui.ctx());
        let width = (ui.available_width() * 0.65).clamp(180.0, 430.0);
        ui.allocate_ui_with_layout(
            egui::vec2(width, 20.0),
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if widgets::icon_button(ui, Icon::Close, "Stop analysis", 20.0).clicked()
                    && let Some(worker) = &self.worker
                {
                    worker.cancel();
                }
                progress_bar(ui, progress, p);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(progress_label(progress))
                            .size(11.5)
                            .color(p.text3),
                    )
                    .truncate(),
                )
                .on_hover_text(progress_label(progress));
            },
        );
    }
}

fn progress_label(progress: &AnalysisProgress) -> String {
    match progress.total {
        Some(total) => {
            let percent = if total == 0 {
                100
            } else {
                progress.completed.saturating_mul(100) / total
            };
            format!(
                "{} · {} / {total} ({percent}%)",
                progress.label, progress.completed
            )
        }
        None if progress.completed > 0 => {
            format!("{} · {} scanned", progress.label, progress.completed)
        }
        None => progress.label.clone(),
    }
}

fn progress_bar(ui: &mut egui::Ui, progress: &AnalysisProgress, palette: Palette) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(96.0, 4.0), Sense::hover());
    ui.painter().rect_filled(rect, 2.0, palette.sel);
    let filled = match progress.total {
        Some(total) => {
            let fraction = if total == 0 {
                1.0
            } else {
                (progress.completed as f64 / total as f64).clamp(0.0, 1.0) as f32
            };
            Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height()))
        }
        None => {
            let position = ui.input(|input| (input.time * 0.7).fract()) as f32;
            let left = rect.left() + position * rect.width() * 0.7;
            ui.ctx().request_repaint_after(Duration::from_millis(100));
            Rect::from_min_size(
                egui::pos2(left, rect.top()),
                egui::vec2(rect.width() * 0.3, rect.height()),
            )
        }
    };
    ui.painter().rect_filled(filled, 2.0, palette.accent);
    response.on_hover_text(progress_label(progress));
}
