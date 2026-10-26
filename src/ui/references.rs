use super::{app::Prism, icons::Icon, theme::Palette, widgets};
use crate::analysis::references::Reference;
use crate::model::View;
use crate::ui::presentation::Selection;
use eframe::egui;

impl Prism {
    pub(super) fn references_view(&mut self, ui: &mut egui::Ui) {
        let Some(references) = &self.workspace.references else {
            ui.label(if self.pending > 0 {
                "Loading references…"
            } else {
                "References are unavailable. Select another function or retry from its menu."
            });
            return;
        };
        let mut destination = None;
        egui::ScrollArea::vertical()
            .id_salt("references")
            .show(ui, |ui| {
                for (heading, rows) in [
                    ("Incoming references", &references.incoming),
                    ("Callees", &references.outgoing),
                ] {
                    widgets::section(ui, &format!("{heading} · {}", rows.len()));
                    if rows.is_empty() {
                        ui.colored_label(Palette::for_context(ui.ctx()).faint, "None found");
                    }
                    for reference in rows {
                        if reference_row(ui, reference).clicked() {
                            destination = Some(reference.address.clone());
                        }
                    }
                    ui.add_space(12.0);
                }
                if references.limited {
                    ui.colored_label(
                        Palette::for_context(ui.ctx()).muted,
                        "Some references may be omitted. Each list is limited to 1,000 entries.",
                    );
                }
            });

        if let Some(address) = destination {
            self.open_function_view(&address, &address, View::Assembly);
        }
    }
}

fn reference_row(ui: &mut egui::Ui, reference: &Reference) -> egui::Response {
    let label = match &reference.name {
        Some(name) => format!("{name} · {}", reference.address),
        None => reference.address.clone(),
    };
    let response = widgets::row_with_font(
        ui,
        Icon::Function,
        &label,
        Selection::Unselected,
        28.0,
        egui::FontId::monospace(12.0),
    );
    match &reference.location {
        Some(location) => response.on_hover_text(format!("Reference at {location}")),
        None => response,
    }
}
