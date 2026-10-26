use super::app::Prism;
use crate::ui::presentation::ReconstructionReadiness;
use eframe::egui;
use serde_json::Value;

mod actions;
mod progress;
mod prompt;

pub(super) struct ReconstructionForm {
    pub name: String,
    pub folder: String,
    pub language: String,
    pub instructions: String,
    pub batch_size: u32,
    pub workers: u32,
    pub status: Option<Value>,
    pub restored: bool,
}

impl Default for ReconstructionForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            folder: String::new(),
            language: "Rust".into(),
            instructions: String::new(),
            batch_size: 16,
            workers: 2,
            status: None,
            restored: false,
        }
    }
}

impl Prism {
    pub(super) fn reconstruction_screen(&mut self, ui: &mut egui::Ui) {
        self.restore_reconstruction();
        ui.heading("Reconstruct project");
        ui.label("Build in resumable generations. Keep earlier output and reuse unchanged work.");
        ui.add_space(12.0);
        if self.reconstruction.status.is_some() {
            self.reconstruction_summary(ui);
        }
        egui::CollapsingHeader::new("Generation settings")
            .default_open(self.reconstruction.status.is_none())
            .show(ui, |ui| self.reconstruction_settings(ui));
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            let ready = !self.chat.busy && self.pending == 0;
            if self.reconstruction.status.is_none() {
                if ui
                    .add_enabled(ready, egui::Button::new("Start reconstruction"))
                    .clicked()
                    && let Err(error) = self.start_reconstruction()
                {
                    self.error = Some(error.to_string());
                }
                if ui
                    .add_enabled(ready, egui::Button::new("Open existing…"))
                    .clicked()
                    && let Some(root) = rfd::FileDialog::new().pick_folder()
                {
                    self.reconstruction_root = Some(root);
                    self.reconstruction_job("reconstruction_status", serde_json::json!({}));
                }
            } else {
                self.reconstruction_controls(ui, ReconstructionReadiness::from(ready));
            }
        });
    }

    fn reconstruction_settings(&mut self, ui: &mut egui::Ui) {
        if self.reconstruction_root.is_none() {
            ui.label("Project name");
            ui.text_edit_singleline(&mut self.reconstruction.name);
            ui.label("Output folder");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.reconstruction.folder);
                if ui.button("Choose…").clicked()
                    && let Some(folder) = rfd::FileDialog::new().pick_folder()
                {
                    self.reconstruction.folder = folder.to_string_lossy().into_owned();
                }
            });
        }
        ui.label("Target language");
        ui.text_edit_singleline(&mut self.reconstruction.language);
        ui.label("Framework, platform and dependency instructions");
        ui.add(
            egui::TextEdit::multiline(&mut self.reconstruction.instructions)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Functions per batch");
            ui.add(egui::DragValue::new(&mut self.reconstruction.batch_size).range(1..=64));
            ui.label("Maximum concurrent batches");
            ui.add(egui::DragValue::new(&mut self.reconstruction.workers).range(1..=8));
        });
        ui.small("Agents that support delegation can use separate workers. Start with two to limit memory use.");
    }
}
