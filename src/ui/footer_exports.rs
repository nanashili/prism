use super::app::Prism;
use super::footer_rows::{self, Lead, Tail};
use super::{icons::Icon, popup};
use crate::error::Result;
use crate::model::View;
use crate::worker::{AnalysisExport, Job};
use eframe::egui;
use std::path::PathBuf;

impl Prism {
    pub(super) fn footer_exports(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.pending == 0, |ui| {
            popup::heading(ui, "This function");
            self.footer_function_exports(ui);
            ui.add_space(8.0);
            popup::heading(ui, "Whole file");
            self.footer_file_exports(ui);
        });
        ui.add_space(8.0);
        self.footer_exports_folder(ui);
    }

    fn footer_function_exports(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(!self.workspace.selected.is_empty(), |ui| {
            for (view, icon, extension) in [
                (View::Pseudocode, Icon::ExportPseudo, "c"),
                (View::Assembly, Icon::ExportAsm, "s"),
            ] {
                if footer_rows::row(ui, Lead::Icon(icon), view.label(), "", Tail::None).clicked() {
                    let filename = format!("{}.{}", safe_name(&self.selected_name()), extension);
                    self.enqueue_analysis_export(
                        filename,
                        AnalysisExport::Function {
                            view,
                            address: self.workspace.selected.clone(),
                        },
                    );
                    ui.close();
                }
            }
        });
    }

    fn footer_file_exports(&mut self, ui: &mut egui::Ui) {
        if footer_rows::row(
            ui,
            Lead::Icon(Icon::Export),
            "All pseudocode",
            "",
            Tail::None,
        )
        .clicked()
        {
            self.enqueue_analysis_export("pseudocode.c".into(), AnalysisExport::AllPseudocode);
            ui.close();
        }
        ui.add_enabled_ui(
            matches!(self.settings.provider.as_str(), "ida" | "hopper"),
            |ui| {
                if footer_rows::row(
                    ui,
                    Lead::Icon(Icon::ExportObjc),
                    "Objective-C classes",
                    "",
                    Tail::None,
                )
                .on_disabled_hover_text("Requires a REA engine with Objective-C inventory support")
                .clicked()
                {
                    self.enqueue_analysis_export("objc.txt".into(), AnalysisExport::ObjectiveC);
                    ui.close();
                }
            },
        );
        self.footer_patched_copy(ui);
    }

    fn footer_patched_copy(&mut self, ui: &mut egui::Ui) {
        let Some(project) = self.workspace.project.as_ref() else {
            return;
        };
        if project.patches.is_empty() {
            return;
        }

        if footer_rows::row(
            ui,
            Lead::Icon(Icon::ExportPatch),
            "Patched copy",
            "",
            Tail::Hint(&format!("{} patches", project.patches.len())),
        )
        .clicked()
        {
            match self.exports_folder() {
                Ok(folder) => self.enqueue(Job::Patch {
                    path: folder.join(format!("{}-patched", safe_name(&project.target.name))),
                    project: project.clone(),
                }),
                Err(error) => self.error = Some(error.to_string()),
            }
            ui.close();
        }
    }

    fn footer_exports_folder(&mut self, ui: &mut egui::Ui) {
        if footer_rows::row(
            ui,
            Lead::Icon(Icon::ExportFolder),
            "Open the exports folder",
            &super::settings_pages::paths::display(&super::settings_pages::paths::exports_dir(
                &self.settings,
            )),
            Tail::None,
        )
        .clicked()
        {
            match self.exports_folder().and_then(|folder| {
                std::fs::create_dir_all(&folder)?;
                url::Url::from_file_path(&folder)
                    .map(String::from)
                    .map_err(|()| crate::error::failure("The exports folder has no file URL"))
            }) {
                Ok(url) => ui.ctx().open_url(egui::OpenUrl::new_tab(url)),
                Err(error) => self.error = Some(error.to_string()),
            }
            ui.close();
        }
    }

    fn enqueue_analysis_export(&mut self, filename: String, kind: AnalysisExport) {
        match self.exports_folder() {
            Ok(folder) => self.enqueue(Job::ExportAnalysis {
                path: folder.join(filename),
                kind,
            }),
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn exports_folder(&self) -> Result<PathBuf> {
        let project = self.workspace.project.as_ref().ok_or("Open a file first")?;
        Ok(super::settings_pages::paths::exports_dir(&self.settings)
            .join(safe_name(&project.target.name)))
    }
}

fn safe_name(name: &str) -> String {
    let name: String = name
        .chars()
        .take(80)
        .map(|character| {
            if character.is_ascii_alphanumeric() || "._-".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() || name == "." || name == ".." {
        "export".into()
    } else {
        name
    }
}
