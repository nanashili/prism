use super::app::Prism;
use crate::comparison::{Ambiguity, Report, Selection};
use crate::error::Result;
use crate::model::Procedure;
use crate::worker::ViewJob;
use eframe::egui;

impl Prism {
    pub(super) fn comparison_view(&mut self, ui: &mut egui::Ui) {
        self.comparison_controls(ui);
        let workspace = &mut self.workspace;
        let Some(report) = workspace.comparison.as_ref() else {
            ui.label("Open the newer version, then choose a baseline. Saved Prism projects carry their confirmed labels, notes and bookmarks.");
            return;
        };
        comparison_summary(ui, report);
        egui::ScrollArea::vertical().show(ui, |ui| {
            review_matches(ui, report, &mut workspace.reviewed_matches);
            review_ambiguities(ui, report, &mut workspace.reviewed_matches);
            unmatched(ui, report);
        });
    }

    fn comparison_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.pending == 0,
                    egui::Button::new("Compare with previous version…"),
                )
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title("Choose baseline app, binary or Prism project")
                    .pick_file()
            {
                self.workspace.comparison = None;
                self.workspace.reviewed_matches.clear();
                self.enqueue_view(ViewJob::Diff(path));
            }
            if ui
                .add_enabled(
                    self.pending == 0 && !self.workspace.reviewed_matches.is_empty(),
                    egui::Button::new("Transfer reviewed labels, notes and bookmarks"),
                )
                .clicked()
            {
                let selections = self.workspace.reviewed_matches.clone();
                if let Err(error) = self.transfer_version_annotations(&selections) {
                    self.report_error(error);
                }
            }
        });
    }

    pub(super) fn transfer_version_annotations(&mut self, selections: &[Selection]) -> Result<()> {
        let report = self
            .workspace
            .comparison
            .as_ref()
            .ok_or("Run a version comparison first")?;
        let project = self.workspace.project.as_ref().ok_or("Open a file first")?;
        let updated = crate::comparison::transfer(report, project, selections)?;
        self.commit_edit(updated)?;
        self.status = format!(
            "Transferred annotations for {} reviewed functions",
            selections.len()
        );
        Ok(())
    }
}

fn comparison_summary(ui: &mut egui::Ui, report: &Report) {
    ui.label(format!(
        "{} → {} · {} matches · {} ambiguous groups · {} possible additions · {} possible removals · {} unknown",
        report.source_name,
        report.target_name,
        report.matches.len(),
        report.ambiguous.len(),
        report.target_only.len(),
        report.source_only.len(),
        report.source_unknown.len() + report.target_unknown.len(),
    ));
    ui.label("Review each pair before transferring. Changed bytes and structural similarity need investigation. Listing spans follow provider inventory boundaries; they may include padding or undiscovered functions. Copied discoveries need revalidation. Conflicting annotations stop the entire transfer; Undo restores the previous project.");
}

fn review_matches(ui: &mut egui::Ui, report: &Report, reviewed: &mut Vec<Selection>) {
    for index in page(ui, "matched-functions", report.matches.len()) {
        let item = &report.matches[index];
        let selection = Selection {
            source: item.source.address.clone(),
            target: item.target.address.clone(),
        };
        let mut selected = contains(reviewed, &selection);
        let label = format!(
            "{} {} → {} {} · {:?} · {} · {:?}",
            item.source.address,
            source_name(report, &item.source),
            item.target.address,
            item.target.name,
            item.method,
            if item.bytes_equal {
                "equal bytes"
            } else {
                "changed bytes"
            },
            item.source_scope
        );
        if ui
            .checkbox(&mut selected, label)
            .on_hover_text(source_note(report, &item.source))
            .changed()
        {
            select(reviewed, selection, selected);
        }
    }
}

fn review_ambiguities(ui: &mut egui::Ui, report: &Report, reviewed: &mut Vec<Selection>) {
    for index in page(ui, "ambiguous-groups", report.ambiguous.len()) {
        let group = &report.ambiguous[index];
        ui.collapsing(
            format!(
                "Ambiguous group {}: {} baseline / {} destination functions",
                index + 1,
                group.source.len(),
                group.target.len()
            ),
            |ui| {
                for row in page(ui, ("ambiguous-sources", index), group.source.len()) {
                    review_ambiguous_source(ui, report, group, &group.source[row], reviewed);
                }
            },
        );
    }
}

fn review_ambiguous_source(
    ui: &mut egui::Ui,
    report: &Report,
    group: &Ambiguity,
    source: &Procedure,
    reviewed: &mut Vec<Selection>,
) {
    ui.horizontal(|ui| {
        ui.label(format!(
            "{} {} →",
            source.address,
            source_name(report, source)
        ))
        .on_hover_text(source_note(report, source));
        let previous = reviewed
            .iter()
            .find(|item| item.source == source.address)
            .map(|item| item.target.clone());
        let mut chosen = previous.clone();
        egui::ComboBox::from_id_salt(("version-match", &source.address))
            .selected_text(chosen.as_deref().unwrap_or("Review destination…"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut chosen, None, "Unconfirmed");
                for index in page(
                    ui,
                    ("ambiguous-targets", &source.address),
                    group.target.len(),
                ) {
                    let target = &group.target[index];
                    ui.selectable_value(
                        &mut chosen,
                        Some(target.address.clone()),
                        format!("{} {}", target.address, target.name),
                    );
                }
            });
        if chosen != previous {
            reviewed.retain(|item| item.source != source.address);
            if let Some(target) = chosen {
                select(
                    reviewed,
                    Selection {
                        source: source.address.clone(),
                        target,
                    },
                    true,
                );
            }
        }
    });
}

fn source_name<'a>(report: &'a Report, function: &'a Procedure) -> &'a str {
    report
        .annotations
        .labels
        .get(&function.address)
        .map_or(function.name.as_str(), String::as_str)
}

fn source_note(report: &Report, function: &Procedure) -> String {
    report.annotations.notes.get(&function.address).map_or_else(
        || "No confirmed baseline note".into(),
        |note| note.chars().take(2048).collect(),
    )
}

fn contains(reviewed: &[Selection], selection: &Selection) -> bool {
    reviewed
        .iter()
        .any(|item| item.source == selection.source && item.target == selection.target)
}

fn select(reviewed: &mut Vec<Selection>, selection: Selection, selected: bool) {
    reviewed.retain(|item| {
        item.source != selection.source && (!selected || item.target != selection.target)
    });
    if selected {
        reviewed.push(selection);
    }
}

fn page(ui: &mut egui::Ui, salt: impl std::hash::Hash, length: usize) -> std::ops::Range<usize> {
    let id = ui.make_persistent_id(salt);
    let mut offset = ui.data_mut(|data| data.get_temp::<usize>(id).unwrap_or(0));
    offset = offset.min(length.saturating_sub(1) / 100 * 100);
    if length > 100 {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(offset > 0, egui::Button::new("Previous"))
                .clicked()
            {
                offset = offset.saturating_sub(100);
            }
            ui.label(format!(
                "{}–{} of {length}",
                offset + 1,
                (offset + 100).min(length)
            ));
            if ui
                .add_enabled(offset + 100 < length, egui::Button::new("Next"))
                .clicked()
            {
                offset += 100;
            }
        });
    }
    ui.data_mut(|data| data.insert_temp(id, offset));
    offset..(offset + 100).min(length)
}

fn unmatched(ui: &mut egui::Ui, report: &Report) {
    for (label, functions) in [
        ("Possible additions", &report.target_only),
        ("Possible removals", &report.source_only),
    ] {
        ui.collapsing(format!("{label} ({})", functions.len()), |ui| {
            for index in page(ui, label, functions.len()) {
                let function = &functions[index];
                ui.label(format!("{} {}", function.address, function.name));
            }
        });
    }
    for (label, unknown) in [
        ("Baseline unknown", &report.source_unknown),
        ("Destination unknown", &report.target_unknown),
    ] {
        ui.collapsing(format!("{label} ({})", unknown.len()), |ui| {
            for index in page(ui, label, unknown.len()) {
                let item = &unknown[index];
                ui.label(format!(
                    "{} {}: {}",
                    item.function.address, item.function.name, item.reason
                ));
            }
        });
    }
}
