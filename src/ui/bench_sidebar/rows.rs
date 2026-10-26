use super::{parts, state::LoadedPage};
use crate::model::View;
use crate::storage::navigation::Row;
use crate::ui::presentation::Selection;
use crate::ui::{app::Prism, icons::Icon, widgets};
use eframe::egui::{self, FontId};

impl Prism {
    pub(super) fn function_tree(&mut self, ui: &mut egui::Ui) {
        let mut visible = Vec::new();
        self.append_function_rows("", 0, &mut visible);
        egui::ScrollArea::vertical()
            .id_salt("function-tree")
            .max_height(ui.available_height().max(90.0))
            .show_rows(ui, 26.0, visible.len(), |ui, range| {
                for index in range {
                    let (parent, row, depth) = &visible[index];
                    ui.push_id((parent, row), |ui| {
                        if *row == usize::MAX {
                            self.function_page_controls(ui, parent);
                        } else if let Some(item) = self
                            .workspace
                            .navigation
                            .pages
                            .get(parent)
                            .and_then(|page| page.page.rows.get(*row))
                            .cloned()
                        {
                            self.function_tree_row(ui, item, *depth);
                        }
                    });
                }
            });
        if visible.is_empty() {
            let loading = self
                .workspace
                .navigation
                .pages
                .get("")
                .is_some_and(|page| !page.ready);
            ui.label(
                egui::RichText::new(if loading {
                    "Loading functions…"
                } else {
                    "No functions match. Use the menu to analyze."
                })
                .small()
                .weak(),
            );
        }
        self.function_page_controls(ui, "");
    }

    fn append_function_rows(
        &self,
        parent: &str,
        depth: usize,
        visible: &mut Vec<(String, usize, usize)>,
    ) {
        let Some(loaded) = self.workspace.navigation.pages.get(parent) else {
            return;
        };
        for (index, row) in loaded.page.rows.iter().enumerate() {
            visible.push((parent.into(), index, depth));
            if let Row::Group { key, .. } = row
                && self.workspace.navigation.expanded.contains(key)
            {
                self.append_function_rows(key, depth + 1, visible);
            }
        }
        if !parent.is_empty()
            && (!loaded.ready || loaded.error.is_some() || loaded.page.more || loaded.offset > 0)
        {
            visible.push((parent.into(), usize::MAX, depth));
        }
    }

    fn function_tree_row(&mut self, ui: &mut egui::Ui, row: Row, depth: usize) {
        match row {
            Row::Group { key, name, count } => {
                let open = self.workspace.navigation.expanded.contains(&key);
                if parts::row(
                    ui,
                    &name,
                    &parts::number(count),
                    parts::RowStyle {
                        icon: (self.workspace.navigation.query.sort
                            == crate::storage::navigation::Sort::Files)
                            .then_some(Icon::Folder),
                        depth,
                        open: Some(open),
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    if open {
                        let prefix = format!("{}/", key.trim_end_matches('/'));
                        self.workspace
                            .navigation
                            .expanded
                            .retain(|parent| parent != &key && !parent.starts_with(&prefix));
                        self.workspace
                            .navigation
                            .pages
                            .retain(|parent, _| parent != &key && !parent.starts_with(&prefix));
                    } else if self.workspace.navigation.pages.len() < 17 {
                        self.workspace.navigation.expanded.insert(key.clone());
                        self.request_sidebar_page(key.clone(), 0);
                    } else {
                        self.status = "Close a function group before opening another.".into();
                    }
                }
            }
            Row::Function {
                procedure,
                location,
                inferred,
            } => {
                let response = ui
                    .horizontal(|ui| {
                        ui.add_space(depth.min(8) as f32 * 14.0);
                        widgets::row_with_font(
                            ui,
                            Icon::Function,
                            &procedure.name,
                            Selection::from(self.workspace.selected == procedure.address),
                            26.0,
                            FontId::monospace(12.0),
                        )
                    })
                    .inner
                    .on_hover_text(format!(
                        "{}\n{}{}\n{}",
                        procedure.address,
                        location,
                        if inferred {
                            " · inferred from surrounding symbols"
                        } else {
                            ""
                        },
                        procedure.kind
                    ));
                if response.clicked() {
                    let view = View::ALL
                        .into_iter()
                        .find(|view| view.label() == self.settings.views.first_view)
                        .unwrap_or(View::Pseudocode);
                    self.preview_function(&procedure.address, &procedure.name, view);
                    if response.double_clicked() {
                        let active = self.workspace.active;
                        self.pin_workbench_tab(active);
                    }
                }
                response.context_menu(|ui| {
                    ui.set_width(248.0);
                    self.function_menu(ui, &procedure.address, &procedure.name);
                });
            }
        }
    }

    fn function_page_controls(&mut self, ui: &mut egui::Ui, parent: &str) {
        let Some(LoadedPage {
            page,
            offset,
            ready,
            error,
        }) = self.workspace.navigation.pages.get(parent)
        else {
            return;
        };
        if !ready {
            ui.label(egui::RichText::new("Loading…").small().weak());
            return;
        }
        if let Some(error) = error {
            if ui
                .small_button("Retry loading functions")
                .on_hover_text(error)
                .clicked()
            {
                self.request_sidebar_page(parent.into(), *offset);
            }
            return;
        }
        if *offset == 0 && !page.more {
            return;
        }
        let mut next = *offset;
        ui.push_id(("function-page", parent), |ui| {
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
            self.request_sidebar_page(parent.into(), next);
        }
    }
}
