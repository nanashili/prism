use super::app::Prism;
use super::{icons::Icon, popup, theme::Palette, widgets};
use crate::model::View;
use eframe::egui::{self, Frame, Stroke};

impl Prism {
    pub(super) fn workbench_status(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        let rect = ui.max_rect();
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0, p.fill),
        );
        Frame::NONE
            .inner_margin(egui::Margin::symmetric(10, 4))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    self.status_details(ui);
                    ui.label(
                        egui::RichText::new(&self.workspace.selected)
                            .monospace()
                            .size(11.5)
                            .color(p.muted),
                    );
                    self.analysis_progress(ui);
                    if self.analysis.is_none() {
                        self.reconstruction_progress(ui);
                    }
                    let status = if self.analysis.is_some() {
                        "Analyzing"
                    } else if self.pending > 0 {
                        "Working…"
                    } else {
                        &self.status
                    };
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(status).size(11.5).color(p.muted))
                                .truncate(),
                        )
                        .on_hover_text(status);
                    });
                });
            });
    }

    fn status_details(&mut self, ui: &mut egui::Ui) {
        let menu = widgets::icon_button(ui, Icon::Panel, "Details", 20.0);
        popup::menu(&menu, 220.0)
            .align(egui::RectAlign::TOP_END)
            .show(|ui| {
                if popup::row(ui, Some(Icon::Note), "Notes and names", "", false).clicked() {
                    self.select_view(View::Notes);
                    ui.close();
                }
                if self.pending > 0
                    && popup::row(ui, Some(Icon::Stop), "Cancel operation", "", false).clicked()
                    && let Some(worker) = &self.worker
                {
                    worker.cancel();
                    ui.close();
                }
            });
    }
}
