use crate::error::Result;
use crate::ui::app::Prism;
use crate::worker::Job;
use eframe::egui;
use serde_json::{Value, json};

impl Prism {
    pub(super) fn restore_reconstruction(&mut self) {
        if self.reconstruction.restored {
            return;
        }
        self.reconstruction.restored = true;
        let Some(project) = &self.workspace.project else {
            return;
        };
        match crate::reconstruction::root(&project.target.sha256, &json!({})) {
            Ok(root) => {
                self.reconstruction_root = Some(root);
                self.reconstruction_job("reconstruction_status", json!({}));
            }
            Err(error) => {
                let missing = error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound);
                if !missing {
                    self.error = Some(format!("Could not restore reconstruction: {error}"));
                }
            }
        }
    }

    pub(super) fn start_reconstruction(&mut self) -> Result<()> {
        if self.workspace.project.is_none() {
            return Err("Open a project first".into());
        }
        if self.reconstruction_root.is_none() {
            self.reconstruction_root = Some(crate::reconstruction::create(
                std::path::Path::new(&self.reconstruction.folder),
                &self.reconstruction.name,
            )?);
        }
        self.start_generation(false);
        Ok(())
    }

    fn start_generation(&mut self, fork: bool) {
        let model = self
            .prompt_option_picks()
            .into_iter()
            .find(|pick| pick.id == crate::agent::options::MODEL_PICK)
            .map(|pick| {
                format!(
                    "{}:{}",
                    crate::agent::picks::key(&self.settings),
                    pick.value
                )
            })
            .unwrap_or_else(|| "default".into());
        self.reconstruction_job("reconstruction_start", json!({
            "language":self.reconstruction.language,"instructions":self.reconstruction.instructions,
            "batch_size":self.reconstruction.batch_size,"workers":self.reconstruction.workers,
            "new_generation":fork,"start_agent":true,
            "model":model,"prompt_version":"prism-modular-v2",
            "adapter_version":"v1","validation_version":"v1",
        }));
    }

    pub(super) fn reconstruction_job(&mut self, tool: &str, mut args: Value) {
        if let Some(root) = &self.reconstruction_root {
            args["root"] = json!(root);
        }
        if tool != "reconstruction_start"
            && let Some(status) = &self.reconstruction.status
        {
            args["generation"] = status["generation"]["id"].clone();
        }
        self.enqueue(Job::Reconstruct {
            tool: tool.into(),
            args,
        });
    }

    pub(super) fn reconstruction_controls(&mut self, ui: &mut egui::Ui, ready: bool) {
        ui.add_enabled_ui(ready, |ui| {
            if ui.button("Continue with agent").clicked() {
                self.reconstruction_job(
                    "reconstruction_control",
                    json!({"action":"resume","start_agent":true}),
                );
            }
            if ui.button("New generation").clicked() {
                self.start_generation(true);
            }
            if ui.button("Retry blocked").clicked() {
                self.reconstruction_job(
                    "reconstruction_control",
                    json!({"action":"retry_blocked"}),
                );
            }
            if ui.button("Build").clicked() {
                self.reconstruction_job("reconstruction_build", json!({}));
            }
            if ui.button("Export source").clicked() {
                self.reconstruction_job("reconstruction_build", json!({"export_only":true}));
            }
            if ui.button("Export report").clicked() {
                self.reconstruction_job("reconstruction_report", json!({}));
            }
        });
        if ui
            .add_enabled(self.pending == 0, egui::Button::new("Pause"))
            .clicked()
        {
            self.reconstruction_job("reconstruction_control", json!({"action":"pause"}));
        }
        if ui.button("Refresh").clicked() {
            self.reconstruction_job("reconstruction_status", json!({}));
        }
        if let Some(root) = &self.reconstruction_root
            && ui.button("Show folder").clicked()
            && let Err(error) = crate::ui::settings_files::reveal(root)
        {
            self.error = Some(error.to_string());
        }
    }
}
