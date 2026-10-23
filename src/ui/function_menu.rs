use super::{app::Prism, icons::Icon, popup};
use crate::model::View;
use crate::worker::Job;
use eframe::egui;

impl Prism {
    pub(super) fn function_menu(&mut self, ui: &mut egui::Ui, address: &str, name: &str) {
        self.function_open_menu(ui, address, name);
        ui.separator();
        self.function_reference_menu(ui, address, name);
        ui.separator();
        self.function_edit_menu(ui, address, name);
        ui.separator();
        popup::submenu(ui, Icon::Copy, "Copy", |ui| {
            for (label, text) in [("Name", name), ("Address", address)] {
                if popup::row(ui, Some(Icon::Copy), label, "", false).clicked() {
                    ui.ctx().copy_text(text.into());
                    ui.close();
                }
            }
            ui.add_enabled_ui(self.pending == 0, |ui| {
                for (view, icon) in [
                    (View::Pseudocode, Icon::Pseudocode),
                    (View::Assembly, Icon::Assembly),
                ] {
                    if popup::row(ui, Some(icon), view.label(), "", false).clicked() {
                        self.enqueue(Job::CopyCode {
                            view,
                            address: address.into(),
                        });
                        ui.close();
                    }
                }
            });
        });
        ui.separator();
        self.function_agent_menu(ui, address, name);
    }

    fn function_reference_menu(&mut self, ui: &mut egui::Ui, address: &str, name: &str) {
        if popup::row(
            ui,
            Some(Icon::References),
            &format!("References to {name}"),
            "",
            false,
        )
        .clicked()
        {
            self.open_function_view(address, name, View::References);
            ui.close();
        }
        if popup::row(ui, Some(Icon::Graph), "Callers and callees", "", false).clicked() {
            self.open_function_view(address, name, View::Graph);
            ui.close();
        }
    }

    fn function_open_menu(&mut self, ui: &mut egui::Ui, address: &str, name: &str) {
        if popup::row(ui, Some(Icon::Function), &format!("Open {name}"), "", false).clicked() {
            self.open_function_view(address, name, View::Pseudocode);
            ui.close();
        }
        if popup::row(ui, Some(Icon::Split), "Open in split", "", false).clicked() {
            self.open_function_in_split(address, name, View::Pseudocode);
            ui.close();
        }
        popup::submenu(ui, Icon::Pseudocode, "Open as", |ui| {
            for (view, icon) in [
                (View::Pseudocode, Icon::Pseudocode),
                (View::Assembly, Icon::Assembly),
                (View::Graph, Icon::Graph),
                (View::Hex, Icon::Hex),
            ] {
                if popup::row(ui, Some(icon), view.label(), "", false).clicked() {
                    self.open_function_view(address, name, view);
                    ui.close();
                }
            }
        });
    }

    fn function_agent_menu(&mut self, ui: &mut egui::Ui, address: &str, name: &str) {
        ui.add_enabled_ui(!self.chat.busy, |ui| {
            for (label, prompt) in [
                (
                    format!("Explain {name}"),
                    format!("Explain what {name} at {address} does, in plain terms."),
                ),
                (
                    "Suggest a better name".into(),
                    format!(
                        "Read {name} at {address} and suggest a clear, specific name with evidence."
                    ),
                ),
            ] {
                if popup::row(ui, Some(Icon::Spark), &label, "", false).clicked() {
                    self.workspace.selected = address.into();
                    self.workspace.agent_closed = false;
                    self.chat.draft = prompt;
                    self.send_prompt();
                    ui.close();
                }
            }
        });
    }

    fn function_edit_menu(&mut self, ui: &mut egui::Ui, address: &str, name: &str) {
        if popup::row(ui, Some(Icon::Rename), &format!("Rename {name}"), "", false).clicked() {
            self.open_function_view(address, name, View::Notes);
            ui.close();
        }
        let marked =
            self.workspace.project.as_ref().is_some_and(|project| {
                project.bookmarks.iter().any(|bookmark| bookmark == address)
            });
        ui.add_enabled_ui(self.pending == 0, |ui| {
            let label = if marked {
                "Remove bookmark"
            } else {
                "Bookmark"
            };
            if popup::row(ui, Some(Icon::Bookmark), label, "", marked).clicked() {
                self.edit_project(|project| {
                    if marked {
                        project.bookmarks.retain(|bookmark| bookmark != address);
                    } else {
                        project.bookmarks.push(address.into());
                    }
                });
                ui.close();
            }
        });
        if popup::row(ui, Some(Icon::Note), "Write a note", "", false).clicked() {
            self.open_function_view(address, name, View::Notes);
            ui.close();
        }
    }
}
