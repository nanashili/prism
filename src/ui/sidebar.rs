use super::app::Prism;
use super::sidebar_rows::project_row;
use super::{icons::Icon, theme::Palette, widgets};
use crate::model::Screen;
use crate::worker::Job;
use eframe::egui::{self, Align2, FontId, Frame, Rect};

impl Prism {
    pub(super) fn sidebar(&mut self, context: &egui::Context, expanded: f32) {
        let p = Palette::for_context(context);
        if self.screen == Screen::Setup {
            return;
        }
        let settings = self.screen == Screen::Settings;
        let workbench = self.screen == Screen::Workbench && self.workspace.project.is_some();
        if expanded == 0.0 {
            return;
        }
        let width = if settings {
            248.0
        } else if workbench {
            268.0
        } else {
            272.0
        };
        let background =
            self.window_effects
                .sidebar(p.window, p.sidebar, self.settings.transparency);
        egui::SidePanel::left("navigation")
            .exact_width(width * expanded)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE.fill(background))
            .show(context, |ui| {
                let rect = ui.max_rect();
                let content =
                    Rect::from_min_max(egui::pos2(rect.right() - width, rect.top()), rect.max);
                let mut column = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("navigation-content")
                        .max_rect(content),
                );
                column.set_clip_rect(ui.clip_rect().intersect(rect));
                if expanded < 1.0 {
                    column.disable();
                }
                Frame::NONE
                    .inner_margin(egui::Margin {
                        left: 10,
                        right: 10,
                        top: 0,
                        bottom: 10,
                    })
                    .show(&mut column, |ui| {
                        ui.set_min_size(ui.available_size());
                        self.sidebar_content(ui, settings, workbench);
                    });
            });
    }

    fn sidebar_content(&mut self, ui: &mut egui::Ui, settings: bool, workbench: bool) {
        if settings {
            self.settings_navigation(ui);
            return;
        }
        egui::TopBottomPanel::bottom("sidebar-footer")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show_inside(ui, |ui| {
                if workbench {
                    self.workbench_footer(ui);
                } else {
                    self.sidebar_footer(ui);
                }
            });
        if workbench {
            self.workbench_navigation(ui);
        } else {
            self.home_navigation(ui);
        }
    }

    fn home_navigation(&mut self, ui: &mut egui::Ui) {
        ui.add_space(46.0);
        let new = widgets::row(ui, Icon::NewChat, "New chat", false, 32.0);
        shortcut(ui, new.rect, "N");
        if new.clicked() {
            self.new_chat();
        }
        let open = widgets::row(ui, Icon::Folder, "Open…", false, 32.0);
        shortcut(ui, open.rect, "O");
        if open.clicked() {
            self.choose_open(false);
        }
        self.project_switcher(ui);
        egui::ScrollArea::vertical()
            .id_salt("home-navigation")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.sidebar_chats(ui);
                widgets::section(ui, "Projects");
                for index in 0..self.recent.len() {
                    let recent = &self.recent[index];
                    let selected = self
                        .workspace
                        .project
                        .as_ref()
                        .is_some_and(|project| project.target.sha256 == recent.id);
                    if project_row(ui, recent, selected).clicked() {
                        self.enqueue(Job::Open(self.recent[index].path.clone()));
                    }
                }
                if self.recent.is_empty() {
                    sidebar_empty(ui, "Open an app or binary to start a project.");
                }
            });
    }
}

fn sidebar_empty(ui: &mut egui::Ui, text: &str) {
    Frame::NONE
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(12.5)
                    .color(Palette::for_context(ui.ctx()).faint),
            );
        });
}

fn shortcut(ui: &egui::Ui, row: Rect, key: &str) {
    let p = Palette::for_context(ui.ctx());
    for (index, text) in [
        key,
        if cfg!(target_os = "macos") {
            "⌘"
        } else {
            "Ctrl"
        },
    ]
    .iter()
    .enumerate()
    {
        let rect = Rect::from_center_size(
            egui::pos2(row.right() - 17.0 - index as f32 * 22.0, row.center().y),
            egui::vec2(19.0, 19.0),
        );
        ui.painter().rect_filled(rect, 5.0, p.fill);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(11.0),
            p.muted,
        );
    }
}
