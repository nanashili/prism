use crate::ui::app::Prism;
use eframe::egui;
use serde_json::Value;

impl Prism {
    pub(in crate::ui) fn receive_reconstruction(
        &mut self,
        project: &str,
        status: Value,
        start_agent: bool,
    ) {
        if self
            .workspace
            .project
            .as_ref()
            .is_none_or(|current| current.target.sha256 != project)
        {
            return;
        }
        let changed = self.reconstruction.status.as_ref().is_none_or(|previous| {
            previous["root"] != status["root"]
                || previous["generation"]["id"] != status["generation"]["id"]
        });
        self.reconstruction_root = status["root"].as_str().map(Into::into);
        if changed {
            self.reconstruction.language = status["generation"]["language"]
                .as_str()
                .unwrap_or("Rust")
                .into();
            self.reconstruction.instructions = status["generation"]["instructions"]
                .as_str()
                .unwrap_or_default()
                .into();
            self.reconstruction.batch_size =
                status["generation"]["batch_size"].as_u64().unwrap_or(16) as u32;
            self.reconstruction.workers =
                status["generation"]["workers"].as_u64().unwrap_or(2) as u32;
        }
        self.reconstruction.status = Some(status);
        if start_agent && !self.chat.busy {
            self.send_reconstruction_prompt();
        }
    }

    pub(super) fn reconstruction_summary(&mut self, ui: &mut egui::Ui) {
        let Some(status) = self.reconstruction.status.as_ref() else {
            return;
        };
        let generation = &status["generation"];
        ui.label(format!(
            "Generation {} · {} · revision {}",
            generation["id"],
            generation["state"].as_str().unwrap_or_default(),
            generation["revision"]
        ));
        let counts = &status["counts"];
        let generated = count(counts, "generated");
        let reused = count(counts, "reused");
        let total = count(counts, "total");
        ui.add(
            egui::ProgressBar::new(fraction(generated + reused, total))
                .desired_width(ui.available_width())
                .text(format!(
                    "{} / {total} units have source",
                    generated + reused
                )),
        );
        ui.label(format!(
            "{generated} generated · {reused} reused · {} active · {} blocked · {} files",
            count(counts, "leased"),
            count(counts, "blocked"),
            status["files"]
        ));
        ui.small("Source counts are agent reports. Build success does not establish behavioral equivalence.");
        if let Some(root) = &self.reconstruction_root {
            ui.label(root.display().to_string());
        }
        if !status["latest_build"].is_null() {
            let build = &status["latest_build"];
            let outcome = match build["succeeded"].as_bool() {
                Some(true) => "passed",
                Some(false) => "failed",
                None => "exported or unfinished",
            };
            ui.label(format!(
                "Build {} · revision {} · {outcome}",
                build["id"], build["revision"]
            ));
            ui.collapsing("Build details", |ui| {
                ui.label(build["snapshot"].as_str().unwrap_or_default());
                ui.monospace(build["diagnostics"].as_str().unwrap_or_default());
            });
        }
        ui.collapsing("Generation history", |ui| {
            for item in status["history"].as_array().into_iter().flatten() {
                ui.label(format!(
                    "Generation {} · {} · {} · revision {}",
                    item["id"],
                    item["language"].as_str().unwrap_or_default(),
                    item["state"].as_str().unwrap_or_default(),
                    item["revision"]
                ));
            }
        });
        ui.add_space(12.0);
    }

    pub(in crate::ui) fn reconstruction_progress(&self, ui: &mut egui::Ui) {
        let Some(status) = &self.reconstruction.status else {
            return;
        };
        if status["generation"]["state"] != "active" {
            return;
        }
        let counts = &status["counts"];
        let done = count(counts, "generated") + count(counts, "reused");
        let total = count(counts, "total");
        if done >= total {
            return;
        }
        ui.add(egui::ProgressBar::new(fraction(done, total)).desired_width(96.0))
            .on_hover_text(format!(
                "Reconstruction generation {}: {done}/{total} source units, {} blocked",
                status["generation"]["id"],
                count(counts, "blocked")
            ));
        ui.small(format!("Reconstructing {done}/{total}"));
    }
}

fn count(value: &Value, key: &str) -> u64 {
    value[key].as_u64().unwrap_or_default()
}

fn fraction(done: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (done as f64 / total as f64) as f32
    }
}
