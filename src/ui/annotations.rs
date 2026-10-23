use super::app::Prism;
use crate::model::View;
use crate::worker::{Job, ViewJob};
use eframe::egui;

impl Prism {
    pub(super) fn annotation_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Notes and names");
        ui.label(format!("Function: {}", self.workspace.selected));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.workspace.rename).char_limit(256));
            if ui
                .add_enabled(self.pending == 0, egui::Button::new("Rename"))
                .clicked()
            {
                let address = self.workspace.selected.clone();
                let name = self.workspace.rename.trim().to_owned();
                if !address.is_empty() && !name.is_empty() {
                    self.edit_project(|project| {
                        project.renamed.insert(address, name);
                    });
                }
            }
            if ui
                .add_enabled(self.pending == 0, egui::Button::new("Bookmark"))
                .clicked()
            {
                let address = self.workspace.selected.clone();
                self.edit_project(|project| {
                    if project.bookmarks.contains(&address) {
                        project.bookmarks.retain(|entry| entry != &address);
                    } else {
                        project.bookmarks.push(address);
                    }
                });
            }
        });
        ui.add(
            egui::TextEdit::multiline(&mut self.workspace.note)
                .char_limit(16_384)
                .desired_rows(12)
                .desired_width(f32::INFINITY),
        );
        if ui
            .add_enabled(self.pending == 0, egui::Button::new("Save note"))
            .clicked()
        {
            let address = self.workspace.selected.clone();
            let note = self.workspace.note.clone();
            self.edit_project(|project| {
                project.notes.insert(address, note);
            });
        }
        ui.add_space(24.0);
        ui.label("Engine tools");
        ui.horizontal(|ui| {
            ui.label("Tool");
            ui.text_edit_singleline(&mut self.tool_name);
        });
        ui.add(
            egui::TextEdit::multiline(&mut self.tool_args)
                .code_editor()
                .desired_rows(4),
        );
        if ui.button("Run tool").clicked() {
            match serde_json::from_str(&self.tool_args) {
                Ok(args) => {
                    self.workspace.view = View::Pseudocode;
                    self.enqueue_view(ViewJob::Tool {
                        name: self.tool_name.clone(),
                        args,
                    });
                }
                Err(error) => self.error = Some(format!("Invalid tool arguments: {error}")),
            }
        }
    }

    pub(super) fn edit_project(&mut self, operation: impl FnOnce(&mut crate::model::Project)) {
        let Some(mut project) = self.workspace.project.clone() else {
            return;
        };
        operation(&mut project);
        if let Err(error) = self.commit_edit(project) {
            self.error = Some(error.to_string());
        }
    }

    pub(super) fn apply_annotation_tool(
        &mut self,
        tool: &str,
        args: &serde_json::Value,
    ) -> crate::error::Result<()> {
        use crate::error::failure;
        let mut project = self.workspace.project.clone().ok_or("Open a file first")?;
        match tool {
            "rename_function" => {
                let address =
                    self.resolve_function(args["function"].as_str().ok_or("Missing function")?)?;
                let name = args["new_name"]
                    .as_str()
                    .filter(|name| !name.trim().is_empty())
                    .ok_or("Missing new name")?;
                if name.len() > 1024 {
                    return Err(failure("Names are limited to 1024 bytes"));
                }
                project.renamed.insert(address, name.into());
            }
            "add_note" => {
                let address =
                    self.resolve_function(args["function"].as_str().ok_or("Missing function")?)?;
                let note = args["note"].as_str().ok_or("Missing note")?;
                if note.len() > 64 * 1024 {
                    return Err(failure("Notes are limited to 64 KiB"));
                }
                if note.is_empty() {
                    project.notes.remove(&address);
                } else {
                    project.notes.insert(address, note.into());
                }
            }
            _ => {
                let address = args["address"].as_str().ok_or("Missing address")?;
                if !project.bookmarks.iter().any(|entry| entry == address) {
                    project.bookmarks.push(address.into());
                }
            }
        }
        self.commit_edit(project)
    }

    fn commit_edit(&mut self, project: crate::model::Project) -> crate::error::Result<()> {
        if self.pending > 0 {
            return Err("Wait for the current operation before editing the project".into());
        }
        self.store.save_project(&project)?;
        self.publish_edit(project);
        Ok(())
    }

    pub(super) fn publish_edit(&mut self, project: crate::model::Project) {
        self.workspace.navigation.invalidate();
        if let Some(previous) = self.workspace.project.replace(project) {
            self.workspace.undo.push(previous);
        }
        while self.workspace.undo.len() > 20
            || self.workspace.undo.iter().map(edit_size).sum::<usize>() > 8 * 1024 * 1024
        {
            self.workspace.undo.remove(0);
        }
    }

    pub(super) fn undo_edit(&mut self) {
        if self.pending > 0 {
            self.error = Some("Wait for the current operation before undoing an edit".into());
            return;
        }
        let Some(mut project) = self.workspace.undo.last().cloned() else {
            return;
        };
        project.source_document = self
            .workspace
            .project
            .as_ref()
            .and_then(|current| current.source_document.clone());
        match self.store.save_project(&project) {
            Ok(()) => {
                self.workspace.undo.pop();
                self.workspace.project = Some(project);
                self.workspace.navigation.invalidate();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(super) fn hex_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("File offset");
            let response = ui.add(
                egui::DragValue::new(&mut self.workspace.hex_offset).hexadecimal(8, false, true),
            );
            if response.changed() || ui.button("Read").clicked() {
                self.select_view(View::Hex);
            }
            if ui.button("Next page").clicked() {
                self.workspace.hex_offset = self.workspace.hex_offset.saturating_add(4096);
                self.select_view(View::Hex);
            }
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.workspace.patch)
                    .hint_text("Patch bytes: 90 90 …")
                    .char_limit(12_000),
            );
            if ui
                .add_enabled(self.pending == 0, egui::Button::new("Stage patch"))
                .clicked()
            {
                match crate::project::patch::parse_hex(&self.workspace.patch) {
                    Ok(bytes) => {
                        let offset = self.workspace.hex_offset;
                        self.edit_project(|project| {
                            project.patches.insert(offset, bytes);
                        });
                        self.workspace.patch.clear();
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
            if ui.button("Write patched copy…").clicked()
                && let Some(project) = self.workspace.project.clone()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title("Write patched copy")
                    .save_file()
            {
                self.enqueue(Job::Patch { path, project });
            }
        });
        ui.label("Patches use file offsets and are applied only to the exported copy.");
    }
}

fn edit_size(project: &crate::model::Project) -> usize {
    let names: usize = project
        .renamed
        .iter()
        .map(|(key, value)| key.len() + value.len())
        .sum();
    let notes: usize = project
        .notes
        .iter()
        .map(|(key, value)| key.len() + value.len())
        .sum();
    let patches: usize = project.patches.values().map(Vec::len).sum();
    names + notes + patches + project.bookmarks.iter().map(String::len).sum::<usize>()
}
