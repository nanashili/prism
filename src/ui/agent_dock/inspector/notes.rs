use crate::{
    error::Result,
    ui::{app::Prism, icons::Icon, theme::Palette},
};
use eframe::egui::{self, Color32, Rect, Sense};
use std::time::{Duration, Instant};

const TAGS: [(&str, &str); 7] = [
    ("None", ""),
    ("Red", "#e06c75"),
    ("Orange", "#d19a66"),
    ("Yellow", "#e5c07b"),
    ("Green", "#98c379"),
    ("Blue", "#61afef"),
    ("Purple", "#c678dd"),
];

impl Prism {
    pub(super) fn inspector_notes(&mut self, ui: &mut egui::Ui) {
        super::heading(ui, "Tag and note");
        self.inspector_colour_tags(ui);
        ui.add_space(10.0);
        let note = ui.add(
            egui::TextEdit::multiline(&mut self.chat_tabs.inspection.note)
                .char_limit(16_384)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .hint_text("What this function does, in your words"),
        );
        if note.changed() {
            self.chat_tabs.inspection.save_at = Some(Instant::now() + Duration::from_millis(400));
        }
        if note.lost_focus()
            && let Err(error) = self.save_inspector_note()
        {
            self.report_error(error);
        }
    }

    fn inspector_colour_tags(&mut self, ui: &mut egui::Ui) {
        let selected = self
            .workspace
            .project
            .as_ref()
            .and_then(|project| project.imported.get("prism-color-tags"))
            .and_then(|tags| tags.get(&self.workspace.selected))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned();
        let mut picked = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for (name, hex) in TAGS {
                let p = Palette::for_context(ui.ctx());
                let colour = if hex.is_empty() {
                    p.fill
                } else {
                    Color32::from_hex(hex).unwrap_or(p.fill)
                };
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::click());
                ui.painter().circle_filled(rect.center(), 10.0, colour);
                if selected == hex {
                    Icon::Check.paint(
                        ui,
                        Rect::from_center_size(rect.center(), egui::vec2(12.0, 12.0)),
                        if hex.is_empty() {
                            p.text
                        } else {
                            Color32::WHITE
                        },
                    );
                }
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::RadioButton,
                        ui.is_enabled(),
                        selected == hex,
                        name,
                    )
                });
                if response.on_hover_text(name).clicked() {
                    picked = Some(hex);
                }
            }
        });
        if let Some(colour) = picked {
            let address = self.workspace.selected.clone();
            self.edit_project(|project| {
                let tags = std::sync::Arc::make_mut(&mut project.imported)
                    .entry("prism-color-tags".into())
                    .or_insert_with(|| serde_json::json!({}));
                if let Some(tags) = tags.as_object_mut() {
                    if colour.is_empty() {
                        tags.remove(&address);
                    } else {
                        tags.insert(address, colour.into());
                    }
                }
            });
        }
    }

    pub(in crate::ui) fn save_inspector_note(&mut self) -> Result<()> {
        let inspection = &self.chat_tabs.inspection;
        if inspection.save_at.is_none() {
            return Ok(());
        }
        let Some(key) = inspection.project.as_deref() else {
            return Ok(());
        };
        let current = self
            .workspace
            .project
            .as_ref()
            .filter(|project| project.target.sha256 == key);
        let mut project = match current {
            Some(project) => project.clone(),
            None => self.store.project(key)?,
        };
        if inspection.note.is_empty() {
            project.notes.remove(&inspection.address);
        } else {
            project
                .notes
                .insert(inspection.address.clone(), inspection.note.clone());
        }
        self.store.save_project(&project)?;
        if current.is_some() {
            self.publish_edit(project);
        }
        self.chat_tabs.inspection.save_at = None;
        Ok(())
    }
}
