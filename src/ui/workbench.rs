use super::app::Prism;
use crate::model::View;
use crate::worker::Job;
use eframe::egui::{self, Frame};

impl Prism {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn workbench(&mut self, ui: &mut egui::Ui) {
        if self.workspace.project.is_none() {
            ui.label("Open an app or binary to use the workbench.");
            return;
        }
        egui::TopBottomPanel::bottom("workbench-status")
            .exact_height(28.0)
            .show_separator_line(false)
            .frame(Frame::NONE)
            .show_inside(ui, |ui| self.workbench_status(ui));
        egui::TopBottomPanel::top("workbench-header")
            .exact_height(44.0)
            .show_separator_line(false)
            .frame(Frame::NONE)
            .show_inside(ui, |ui| self.workbench_header(ui));
        self.workbench_columns(ui);
    }

    pub(super) fn workbench_pane(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        self.function_tab(ui);
        if self.workspace.tab_closed {
            return;
        }
        let identity = self.workspace.tabs[self.workspace.active].identity;
        ui.push_id(("workbench-tab", identity), |ui| {
            self.workbench_tab_body(ui);
        });
    }

    fn workbench_tab_body(&mut self, ui: &mut egui::Ui) {
        if matches!(
            self.workspace.view,
            View::Pseudocode | View::Assembly | View::Graph | View::Hex
        ) && !(self.workspace.view == View::Assembly && self.workspace.selected.is_empty())
        {
            self.function_mode_bar(ui);
        }
        Frame::NONE
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| match self.workspace.view {
                View::Assembly if self.workspace.selected.is_empty() => {
                    self.listing_controls(ui);
                    self.code(ui);
                }
                View::Graph => self.graph_view(ui),
                View::References => self.references_view(ui),
                View::Notes => self.annotation_view(ui),
                View::Hex => {
                    self.hex_controls(ui);
                    self.code(ui);
                }
                View::Diff => self.comparison_view(ui),
                View::Debug => self.debug_screen(ui),
                View::Reconstruct => self.reconstruction_screen(ui),
                _ => self.code(ui),
            });
    }

    fn listing_controls(&mut self, ui: &mut egui::Ui) {
        let mut offset = self.workspace.hex_offset;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Whole-file listing").strong());
            if ui
                .add_enabled(offset > 0, egui::Button::new("Previous"))
                .clicked()
            {
                offset = offset.saturating_sub(5);
            }
            if ui
                .add_enabled(
                    self.pending == 0
                        && offset.saturating_add(5) < self.workspace.navigation.total() as u64,
                    egui::Button::new("Next 5"),
                )
                .clicked()
            {
                offset = offset.saturating_add(5);
            }
            ui.label("Function");
            ui.add(egui::DragValue::new(&mut offset).range(0..=999_999));
        });
        if offset != self.workspace.hex_offset {
            self.workspace.hex_offset = offset;
            self.load_view(View::Assembly);
        }
    }

    pub(super) fn export_text(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export current view")
            .set_file_name("analysis.txt")
            .save_file()
        {
            self.enqueue(Job::Export {
                path,
                text: self.workspace.text.clone(),
            });
        }
    }
}
