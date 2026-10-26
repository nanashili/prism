use super::parts;
use crate::model::View;
use crate::ui::presentation::Selection;
use crate::ui::{app::Prism, icons::Icon, popup};
use eframe::egui;

impl Prism {
    pub(super) fn sidebar_file_views(&mut self, ui: &mut egui::Ui) {
        for (view, icon, label) in [
            (View::Assembly, Icon::Assembly, "Listing"),
            (View::Reconstruct, Icon::Spark, "Reconstruct project"),
            (View::Strings, Icon::Strings, "Strings"),
            (View::Imports, Icon::Imports, "Imports"),
            (View::Segments, Icon::Segments, "Segments"),
        ] {
            if !self.view_relevant(view) {
                continue;
            }
            let counts = self.workspace.navigation.counts();
            let count = counts
                .and_then(|counts| match view {
                    View::Strings => counts.strings,
                    View::Imports => counts.imports,
                    View::Segments => counts.segments,
                    _ => None,
                })
                .map(parts::number)
                .unwrap_or_default();
            let response = parts::row(
                ui,
                label,
                &count,
                parts::RowStyle {
                    icon: Some(icon),
                    selected: self.workspace.view == view
                        && (view != View::Assembly || self.workspace.selected.is_empty()),
                    ..Default::default()
                },
            );
            if response.clicked() {
                self.open_sidebar_view(view, label, false);
            }
            if view != View::Reconstruct {
                response.context_menu(|ui| {
                    for split in [false, true] {
                        if popup::row(
                            ui,
                            Some(if split { Icon::Split } else { icon }),
                            if split { "Open in split" } else { "Open" },
                            "",
                            Selection::Unselected,
                        )
                        .clicked()
                        {
                            self.open_sidebar_view(view, label, split);
                            ui.close();
                        }
                    }
                    if let Some(warning) = self
                        .workspace
                        .navigation
                        .counts()
                        .and_then(|counts| counts.warning.as_deref())
                    {
                        ui.separator();
                        ui.label(egui::RichText::new(warning).small().weak());
                    }
                });
            }
        }
    }

    fn open_sidebar_view(&mut self, view: View, label: &str, split: bool) {
        if split {
            self.open_function_in_split("", label, view);
        } else {
            self.open_function_view("", label, view);
        }
        if view == View::Reconstruct {
            self.workspace.agent_closed = false;
        }
    }
}
