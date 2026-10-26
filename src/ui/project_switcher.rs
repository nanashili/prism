use super::app::Prism;
use super::{icons::Icon, popup, theme::Palette, widgets};
use crate::ui::presentation::{OpenTarget, Selection};
use crate::worker::Job;
use eframe::egui::{self, Align2, FontId, Rect, Sense};

impl Prism {
    pub(super) fn project_switcher(&mut self, ui: &mut egui::Ui) {
        let Some(project) = &self.workspace.project else {
            return;
        };
        let title = project.target.name.clone();
        let arch = project.target.arch.clone();
        let p = Palette::for_context(ui.ctx());
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 58.0), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(rect, 10.0, p.sel);
        }
        let tile = Rect::from_min_size(rect.min + egui::vec2(8.0, 10.0), egui::vec2(34.0, 34.0));
        super::file_icons::paint(ui, project.target.icon_path(), tile);
        let clip = Rect::from_min_max(
            rect.min + egui::vec2(52.0, 0.0),
            rect.right_bottom() - egui::vec2(28.0, 0.0),
        );
        ui.painter().with_clip_rect(clip).text(
            clip.min + egui::vec2(0.0, 18.0),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional(15.0),
            p.text,
        );
        let counts = self.workspace.navigation.counts();
        let format = counts.map_or("", |counts| counts.format.as_str());
        let architecture = arch.strip_prefix("Mach-O ").unwrap_or(&arch);
        let architecture = if architecture == format {
            ""
        } else {
            architecture
        };
        let mut left = clip.left();
        for label in [architecture, format]
            .into_iter()
            .filter(|label| !label.is_empty())
        {
            let text =
                ui.painter()
                    .layout_no_wrap(label.into(), FontId::proportional(11.0), p.muted);
            let pill = Rect::from_min_size(
                egui::pos2(left, rect.top() + 30.0),
                egui::vec2(text.size().x + 12.0, 18.0),
            );
            ui.painter()
                .with_clip_rect(clip)
                .rect_filled(pill, 4.0, p.sel);
            ui.painter().with_clip_rect(clip).galley(
                pill.left_center() + egui::vec2(6.0, -text.size().y * 0.5),
                text,
                p.muted,
            );
            left = pill.right() + 5.0;
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Switch project")
        });
        Icon::UpDown.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(rect.right() - 15.0, rect.center().y),
                egui::vec2(14.0, 14.0),
            ),
            p.muted,
        );
        popup::menu(&response, 260.0)
            .align(egui::RectAlign::BOTTOM_START)
            .show(|ui| {
                popup::heading(ui, "Recent projects");
                for index in 0..self.recent.len() {
                    let recent = &self.recent[index];
                    let selected = self
                        .workspace
                        .project
                        .as_ref()
                        .is_some_and(|project| project.target.sha256 == recent.id);
                    let row = widgets::row_with_icon(
                        ui,
                        &recent.title,
                        Selection::from(selected),
                        32.0,
                        FontId::proportional(13.5),
                        |ui, rect| {
                            super::file_icons::paint(ui, &recent.original, rect);
                        },
                    );
                    if row.clicked() {
                        self.enqueue(Job::Open(self.recent[index].path.clone()));
                        ui.close();
                    }
                }
                ui.add_space(8.0);
                if popup::row(
                    ui,
                    Some(Icon::New),
                    "Open another file…",
                    if cfg!(target_os = "macos") {
                        "⌘O"
                    } else {
                        "Ctrl+O"
                    },
                    Selection::Unselected,
                )
                .clicked()
                {
                    self.choose_open(OpenTarget::File);
                    ui.close();
                }
            });
    }
}
