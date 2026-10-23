use super::parts;
use crate::storage::resources::ResourceEntry;
use crate::ui::{app::Prism, icons::Icon, popup};
use eframe::egui;

impl Prism {
    pub(super) fn resource_section(&mut self, ui: &mut egui::Ui) {
        if !self.workspace.navigation.resources.pages.contains_key("") {
            self.request_resource_page(String::new(), 0);
        }
        let root = self.workspace.navigation.resources.pages.get("");
        if root.is_some_and(|(page, _)| !page.available) {
            return;
        }
        let total = root.map_or(0, |(page, _)| page.total);
        let count = if total == 0 {
            String::new()
        } else {
            format!("{} files", parts::number(total))
        };
        if !self.sidebar_heading(ui, "resources", "Resources", &count) {
            return;
        }
        let mut visible = Vec::new();
        self.append_resource_rows("", 0, &mut visible);
        egui::ScrollArea::vertical()
            .id_salt("resource-tree")
            .max_height(220.0)
            .show_rows(ui, 26.0, visible.len(), |ui, range| {
                for index in range {
                    let (parent, row, depth) = &visible[index];
                    ui.push_id(("resource", parent, row), |ui| {
                        if *row == usize::MAX {
                            self.resource_paging(ui, parent);
                        } else if let Some(entry) = self
                            .workspace
                            .navigation
                            .resources
                            .pages
                            .get(parent)
                            .and_then(|(page, _)| page.entries.get(*row))
                            .cloned()
                        {
                            self.resource_row(ui, &entry, *depth);
                        }
                    });
                }
            });
    }

    fn append_resource_rows(
        &self,
        parent: &str,
        depth: usize,
        visible: &mut Vec<(String, usize, usize)>,
    ) {
        let resources = &self.workspace.navigation.resources;
        let Some((page, offset)) = resources.pages.get(parent) else {
            return;
        };
        for (index, entry) in page.entries.iter().enumerate() {
            visible.push((parent.into(), index, depth));
            if entry.folder && resources.expanded.contains(&entry.path) {
                self.append_resource_rows(&entry.path, depth + 1, visible);
            }
        }
        if resources.pending.contains(parent)
            || resources.errors.contains_key(parent)
            || page.more
            || *offset > 0
        {
            visible.push((parent.into(), usize::MAX, depth));
        }
    }

    fn resource_row(&mut self, ui: &mut egui::Ui, entry: &ResourceEntry, depth: usize) {
        let open = self
            .workspace
            .navigation
            .resources
            .expanded
            .contains(&entry.path);
        let address = format!("resource:{}", entry.path);
        let icon = if entry.folder {
            Icon::Folder
        } else {
            Icon::Source
        };
        let response = parts::row(
            ui,
            &entry.name,
            "",
            parts::RowStyle {
                icon: Some(icon),
                selected: self.workspace.selected == address,
                depth,
                open: entry.folder.then_some(open),
                compact: true,
            },
        );
        if response.clicked() {
            if entry.folder {
                self.toggle_resource_folder(&entry.path, open);
            } else {
                self.open_function_view(&address, &entry.name, crate::model::View::Resources);
            }
        }
        if !entry.folder {
            response.context_menu(|ui| {
                for split in [false, true] {
                    let label = if split { "Open in split" } else { "Open" };
                    if popup::row(ui, Some(icon), label, "", false).clicked() {
                        if split {
                            self.open_function_in_split(
                                &address,
                                &entry.name,
                                crate::model::View::Resources,
                            );
                        } else {
                            self.open_function_view(
                                &address,
                                &entry.name,
                                crate::model::View::Resources,
                            );
                        }
                        ui.close();
                    }
                }
                if popup::row(ui, Some(Icon::Copy), "Copy resource path", "", false).clicked() {
                    ui.ctx().copy_text(entry.path.clone());
                    ui.close();
                }
            });
        }
    }

    fn resource_paging(&mut self, ui: &mut egui::Ui, parent: &str) {
        let resources = &self.workspace.navigation.resources;
        if resources.pending.contains(parent) {
            ui.label(egui::RichText::new("Loading resources…").small().weak());
            return;
        }
        if let Some(error) = resources.errors.get(parent) {
            if ui
                .small_button("Retry loading resources")
                .on_hover_text(error)
                .clicked()
            {
                self.request_resource_page(parent.into(), 0);
            }
            return;
        }
        let Some((page, offset)) = self.workspace.navigation.resources.pages.get(parent) else {
            return;
        };
        if *offset == 0 && !page.more {
            return;
        }
        let mut next = *offset;
        ui.push_id(("resource-page", parent), |ui| {
            ui.horizontal(|ui| {
                if *offset > 0 && ui.small_button("Previous").clicked() {
                    next = offset.saturating_sub(300);
                }
                if page.more && ui.small_button("Next 300").clicked() {
                    next += 300;
                }
            })
        });
        if next != *offset {
            self.request_resource_page(parent.into(), next);
        }
    }
}
