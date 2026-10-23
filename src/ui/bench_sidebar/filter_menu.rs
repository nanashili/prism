use crate::storage::navigation::{Show, Sort};
use crate::ui::{app::Prism, icons::Icon, popup};
use crate::worker::Job;
use eframe::egui;

impl Prism {
    pub(super) fn function_filter_menu(&mut self, ui: &mut egui::Ui) {
        let before = (
            self.workspace.navigation.query.show,
            self.workspace.navigation.query.sort,
        );
        for (show, label) in [
            (Show::All, "All functions"),
            (Show::Yours, "Your code only"),
            (Show::Renamed, "Renamed by you"),
            (Show::Imports, "Imports"),
        ] {
            if popup::row(
                ui,
                None,
                label,
                "",
                self.workspace.navigation.query.show == show,
            )
            .clicked()
            {
                self.workspace.navigation.query.show = show;
                ui.close();
            }
        }
        ui.separator();
        for (sort, label) in [
            (Sort::Files, "Group by package or file"),
            (Sort::Modules, "Group by module"),
            (Sort::Address, "Sort by address"),
            (Sort::Name, "Sort by name"),
        ] {
            if popup::row(
                ui,
                None,
                label,
                "",
                self.workspace.navigation.query.sort == sort,
            )
            .clicked()
            {
                self.workspace.navigation.query.sort = sort;
                ui.close();
            }
        }
        ui.separator();
        if popup::row(ui, Some(Icon::Play), "Analyze functions", "", false).clicked() {
            self.enqueue(Job::Index);
            ui.close();
        }
        if popup::row(ui, Some(Icon::Retry), "Refresh functions", "", false).clicked() {
            self.workspace.navigation.invalidate();
            ui.close();
        }
        if before
            != (
                self.workspace.navigation.query.show,
                self.workspace.navigation.query.sort,
            )
        {
            self.workspace.navigation.invalidate();
            self.save_sidebar_preferences();
        }
    }
}
