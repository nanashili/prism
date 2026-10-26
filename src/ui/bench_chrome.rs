use super::app::Prism;
use super::{icons::Icon, popup, theme::Palette, widgets};
use crate::model::View;
use crate::ui::presentation::{HistoryDirection, SaveDestination, Selection};
use crate::worker::Job;
use eframe::egui::{self, Align2, FontId, Frame, Rect, Sense, Stroke};

impl Prism {
    pub(super) fn workbench_header(&mut self, ui: &mut egui::Ui) {
        Frame::NONE
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if self.workspace.navigation.hidden
                        && widgets::icon_button(ui, Icon::Sidebar, "Show sidebar", 28.0).clicked()
                    {
                        self.workspace.navigation.hidden = false;
                        self.save_sidebar_preferences();
                    }
                    self.history_buttons(ui);
                    self.workbench_crumbs(ui);
                    if header_pill(ui, Icon::Search, "Find").clicked() {
                        ui.ctx().memory_mut(|memory| {
                            memory.request_focus(egui::Id::new("function-filter"));
                        });
                    }
                    if self.view_relevant(View::Debug)
                        && header_pill(ui, Icon::Play, "Debug").clicked()
                    {
                        self.select_view(View::Debug);
                    }
                    let agent = super::agent_picker::label(&self.settings);
                    if header_pill(ui, Icon::Spark, &format!("Ask {agent}")).clicked() {
                        self.workspace.agent_closed = !self.workspace.agent_closed;
                    }
                    self.workbench_menu(ui);
                });
            });
    }

    fn history_buttons(&mut self, ui: &mut egui::Ui) {
        let back = ui
            .add_enabled_ui(self.workspace.history_at > 0, |ui| {
                widgets::icon_button(ui, Icon::Back, "Back", 28.0)
            })
            .inner;
        if back.clicked() {
            self.navigate_history(HistoryDirection::Backward);
        }
        let next = ui
            .add_enabled_ui(
                self.workspace.history_at + 1 < self.workspace.history.len(),
                |ui| widgets::icon_button(ui, Icon::Forward, "Forward", 28.0),
            )
            .inner;
        if next.clicked() {
            self.navigate_history(HistoryDirection::Forward);
        }
    }

    fn workbench_crumbs(&self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        let width = (ui.available_width() - 280.0).max(60.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 30.0), Sense::hover());
        let title = self
            .workspace
            .project
            .as_ref()
            .map_or("", |project| project.target.name.as_str());
        let name = self.selected_name();
        let text = if name.is_empty() {
            title.to_owned()
        } else {
            format!("{title}  ›  {name}")
        };
        if let Some(project) = &self.workspace.project {
            super::file_icons::paint(
                ui,
                project.target.icon_path(),
                Rect::from_center_size(
                    rect.left_center() + egui::vec2(12.0, 0.0),
                    egui::vec2(16.0, 16.0),
                ),
            );
        }
        ui.painter().with_clip_rect(rect).text(
            rect.left_center() + egui::vec2(28.0, 0.0),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(13.0),
            p.muted,
        );
    }

    fn workbench_menu(&mut self, ui: &mut egui::Ui) {
        let response = widgets::icon_button(ui, Icon::More, "Project actions", 26.0);
        popup::menu(&response, 228.0).show(|ui| {
            for (label, icon, action) in [
                ("Analyze functions", Icon::Function, 0),
                ("Save project", Icon::Save, 1),
                ("Export current view…", Icon::Export, 2),
                ("Compare files", Icon::Split, 3),
                ("Notes and names", Icon::Note, 4),
            ] {
                if popup::row(ui, Some(icon), label, "", Selection::Unselected).clicked() {
                    match action {
                        0 => self.enqueue(Job::Index),
                        1 => self.save_document(SaveDestination::Current),
                        2 => self.export_text(),
                        3 => self.select_view(View::Diff),
                        _ => self.select_view(View::Notes),
                    }
                    ui.close();
                }
            }
            if popup::row(
                ui,
                Some(Icon::Copy),
                "Copy current view",
                "",
                Selection::Unselected,
            )
            .clicked()
            {
                ui.ctx().copy_text(self.workspace.text.clone());
                ui.close();
            }
        });
    }

    pub(super) fn selected_name(&self) -> String {
        self.workspace
            .project
            .as_ref()
            .and_then(|project| project.renamed.get(&self.workspace.selected))
            .cloned()
            .or_else(|| {
                self.workspace
                    .rows
                    .iter()
                    .find(|row| row.address == self.workspace.selected)
                    .map(|row| row.name.clone())
            })
            .unwrap_or_else(|| self.workspace.selected.clone())
    }
}

pub(super) fn header_pill(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let width = ui
        .painter()
        .layout_no_wrap(label.into(), FontId::proportional(13.0), p.text3)
        .size()
        .x
        + 41.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), Sense::click());
    ui.painter().rect(
        rect,
        8.0,
        if response.hovered() {
            p.sel
        } else {
            egui::Color32::TRANSPARENT
        },
        Stroke::new(1.0, p.sel),
        egui::StrokeKind::Inside,
    );
    icon.paint(
        ui,
        Rect::from_min_size(rect.min + egui::vec2(10.0, 7.0), egui::vec2(14.0, 14.0)),
        p.text3,
    );
    ui.painter().text(
        rect.min + egui::vec2(31.0, 14.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        p.text3,
    );
    response
}
