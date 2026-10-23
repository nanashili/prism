mod notes;
mod request;

use crate::{
    error::Result,
    ui::{app::Prism, theme::Palette},
};
use eframe::egui::{self, FontId, RichText};
use serde_json::Value;
use std::{sync::mpsc::Receiver, time::Instant};

#[derive(Default)]
pub(in crate::ui) struct Inspection {
    project: Option<String>,
    address: String,
    pending: Option<Receiver<Result<Value>>>,
    data: Value,
    error: Option<String>,
    note: String,
    save_at: Option<Instant>,
}

impl Prism {
    pub(in crate::ui) fn dock_inspector(&mut self, ui: &mut egui::Ui) {
        if self.workspace.selected.is_empty() {
            ui.add_space(12.0);
            ui.colored_label(
                Palette::for_context(ui.ctx()).muted,
                "Select a function to inspect it.",
            );
            return;
        }
        if !self.prepare_inspection() {
            ui.label("The previous note could not be saved. Its text is still retained.");
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("selection-inspector")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(14, 4))
                    .show(ui, |ui| {
                        self.inspector_selection(ui);
                        ui.add_space(18.0);
                        self.inspector_function(ui);
                        ui.add_space(18.0);
                        self.inspector_notes(ui);
                        ui.add_space(18.0);
                        self.inspector_actions(ui);
                    });
            });
    }

    fn inspector_selection(&self, ui: &mut egui::Ui) {
        heading(ui, "Selection");
        let inspection = &self.chat_tabs.inspection;
        pair(ui, "Address", &self.workspace.selected, true);
        let location = &inspection.data["selection"];
        let loading = inspection.pending.is_some();
        let offset = location["offset"]
            .as_u64()
            .map(|offset| format!("0x{offset:x}"));
        pair(
            ui,
            "In file at",
            offset
                .as_deref()
                .unwrap_or(if loading { "Reading…" } else { "Not mapped" }),
            true,
        );
        pair(
            ui,
            "Segment",
            location["segment"].as_str().unwrap_or(if loading {
                "Reading…"
            } else {
                "Unavailable"
            }),
            false,
        );
        pair(
            ui,
            "Bytes",
            location["bytes"]
                .as_str()
                .filter(|bytes| !bytes.is_empty())
                .unwrap_or(if loading { "Reading…" } else { "None" }),
            true,
        );
        if let Some(error) = inspection.data["selection_error"].as_str() {
            ui.label(
                RichText::new(error)
                    .size(11.5)
                    .color(Palette::for_context(ui.ctx()).muted),
            );
        }
    }

    fn inspector_function(&self, ui: &mut egui::Ui) {
        heading(ui, "Function");
        let name = self
            .workspace
            .project
            .as_ref()
            .and_then(|project| project.renamed.get(&self.workspace.selected))
            .or_else(|| {
                self.workspace
                    .rows
                    .iter()
                    .find(|row| row.address == self.workspace.selected)
                    .map(|row| &row.name)
            })
            .unwrap_or(&self.workspace.selected);
        pair(ui, "Name", name, true);
        let inspection = &self.chat_tabs.inspection;
        let references = &inspection.data["references"];
        let callers = references["incoming"]
            .as_array()
            .map(|rows| format!("{} places", rows.len()));
        let calls = references["outgoing"].as_array().map(|rows| {
            let names = rows
                .iter()
                .filter_map(|row| {
                    row["name"]
                        .as_str()
                        .filter(|name| !name.is_empty())
                        .or_else(|| row["address"].as_str())
                })
                .collect::<Vec<_>>();
            if names.is_empty() {
                "Nothing".into()
            } else {
                names.join(", ")
            }
        });
        let pending = if inspection.pending.is_some() {
            "Finding…"
        } else {
            "Unavailable"
        };
        pair(
            ui,
            "Called by",
            callers.as_deref().unwrap_or(pending),
            false,
        );
        pair(ui, "Calls", calls.as_deref().unwrap_or(pending), false);
        if references["limited"] == true {
            ui.label(
                RichText::new("References may be incomplete.")
                    .small()
                    .weak(),
            );
        }
        if let Some(error) = inspection.data["references_error"]
            .as_str()
            .or(inspection.error.as_deref())
        {
            ui.label(RichText::new(error).small().weak());
        }
    }

    fn inspector_actions(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let offset = self.chat_tabs.inspection.data["selection"]["offset"].as_u64();
            if ui
                .add_enabled(offset.is_some(), egui::Button::new("Patch this line…"))
                .clicked()
                && let Some(offset) = offset
            {
                self.open_hex_at(offset);
            }
            if ui
                .add_enabled(!self.chat.busy, egui::Button::new("Ask about this line"))
                .clicked()
            {
                self.ask_inspected_line();
            }
            if ui
                .add_enabled(
                    self.chat_tabs.inspection.pending.is_none(),
                    egui::Button::new("Refresh"),
                )
                .clicked()
            {
                self.request_inspection();
            }
        });
    }

    fn ask_inspected_line(&mut self) {
        let prompt = format!("What does the code at {} do?", self.workspace.selected);
        let draft = std::mem::replace(&mut self.chat.draft, prompt);
        let result = self.prepare_prompt();
        self.chat.draft = draft;
        self.chat_tabs.inspector = false;
        if let Err(error) = result {
            self.report_error(error);
        }
    }
}

fn heading(ui: &mut egui::Ui, title: &str) {
    ui.label(
        RichText::new(title)
            .size(11.5)
            .strong()
            .color(Palette::for_context(ui.ctx()).muted),
    );
    ui.add_space(8.0);
}

fn pair(ui: &mut egui::Ui, label: &str, value: &str, mono: bool) {
    let p = Palette::for_context(ui.ctx());
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [84.0, 22.0],
            egui::Label::new(RichText::new(label).size(12.5).color(p.muted)),
        );
        let text = RichText::new(value)
            .font(if mono {
                FontId::monospace(12.0)
            } else {
                FontId::proportional(12.5)
            })
            .color(p.text3);
        ui.add(egui::Label::new(text).selectable(true).wrap());
    });
}
