mod filter_menu;
mod functions;
mod parts;
mod preferences;
mod resource_rows;
mod resources;
mod rows;
mod state;
mod views;
mod work;

use super::{app::Prism, icons::Icon, widgets};
use eframe::egui;
pub(super) use state::Navigation;

impl Prism {
    pub(super) fn workbench_navigation(&mut self, ui: &mut egui::Ui) {
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 46.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                if widgets::icon_button(ui, Icon::Sidebar, "Hide sidebar", 28.0).clicked() {
                    self.workspace.navigation.hidden = true;
                    self.save_sidebar_preferences();
                }
            },
        );
        self.project_switcher(ui);
        self.ensure_sidebar(ui);
        egui::ScrollArea::vertical()
            .id_salt("workbench-navigation")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.workbench_chats(ui);
                self.saved_work(ui);
                ui.add_space(10.0);
                self.sidebar_file_views(ui);
                self.resource_section(ui);
                let count = parts::number(self.workspace.navigation.total());
                if self.sidebar_heading(ui, "functions", "Functions", &count) {
                    self.function_filter(ui);
                    self.function_tree(ui);
                }
            });
    }

    fn sidebar_heading(
        &mut self,
        ui: &mut egui::Ui,
        id: &'static str,
        title: &str,
        count: &str,
    ) -> bool {
        let mut open = self.sidebar_section_open(id);
        if parts::heading(ui, title, count, &mut open).clicked() {
            self.set_sidebar_section_open(id, open);
        }
        open
    }

    pub(super) fn sidebar_section_open(&self, id: &str) -> bool {
        !self.workspace.navigation.closed.contains(id)
    }

    pub(super) fn set_sidebar_section_open(&mut self, id: &'static str, open: bool) {
        if open {
            self.workspace.navigation.closed.remove(id);
        } else {
            self.workspace.navigation.closed.insert(id);
        }
        self.save_sidebar_preferences();
    }
}
