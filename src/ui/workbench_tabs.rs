use super::{app::Prism, view_tab::ViewTab};
use crate::model::View;
use crate::ui::presentation::TabCloseScope;

impl Prism {
    pub(super) fn preview_function(&mut self, address: &str, name: &str, view: View) {
        self.open_function_tab(address, name, view, true);
    }

    pub(super) fn open_function_view(&mut self, address: &str, name: &str, view: View) {
        self.open_function_tab(address, name, view, false);
    }

    fn open_function_tab(&mut self, address: &str, name: &str, view: View, preview: bool) {
        self.focus_current_pane();
        let existing = self.workspace.tabs.iter().position(|tab| {
            !tab.tab_closed
                && tab.selected == address
                && (tab.view == view || (is_code_view(tab.view) && is_code_view(view)))
        });
        if let Some(index) = existing {
            let switched = self.workspace.select_tab(index);
            self.workspace.preview &= preview;
            self.workspace.rename = name.into();
            if switched || self.workspace.view != view {
                self.select_view(view);
            }
            return;
        }

        let tab = ViewTab {
            selected: address.into(),
            rename: name.into(),
            view,
            preview,
            ..Default::default()
        };
        if let Err(error) = self.workspace.open_tab(tab) {
            self.error = Some(error.to_string());
            return;
        }
        self.select_view(view);
    }

    pub(super) fn select_workbench_tab(&mut self, index: usize) {
        self.focus_current_pane();
        if self.workspace.select_tab(index) {
            self.reload_workbench_tab();
        }
    }

    pub(super) fn close_workbench_tab(&mut self, index: usize) {
        self.focus_current_pane();
        let active = self.workspace.active;
        if self.workspace.close_tab(index) {
            if active == index {
                self.reload_workbench_tab();
            }
        } else if self.workspace.split {
            self.close_workbench_pane();
        } else {
            self.workspace.tabs[0] = ViewTab {
                tab_closed: true,
                ..Default::default()
            };
        }
    }

    pub(super) fn pin_workbench_tab(&mut self, index: usize) {
        self.workspace.tabs[index].preview = false;
    }

    pub(super) fn tab_title(&self, index: usize) -> String {
        let tab = &self.workspace.tabs[index];
        if tab.view == View::Resources && tab.selected.starts_with("resource:") {
            return tab.rename.clone();
        }
        if !is_code_view(tab.view) {
            return tab.view.label().into();
        }
        self.workspace
            .project
            .as_ref()
            .and_then(|project| project.renamed.get(&tab.selected))
            .cloned()
            .or_else(|| (!tab.rename.is_empty()).then(|| tab.rename.clone()))
            .unwrap_or_else(|| {
                if tab.selected.is_empty() {
                    "Listing".into()
                } else {
                    tab.selected.clone()
                }
            })
    }

    pub(super) fn prepare_tool_tab(&mut self, view: View) -> bool {
        if is_code_view(view) || self.workspace.view == view {
            return true;
        }
        if let Some(index) = self.workspace.tabs.iter().position(|tab| {
            !tab.tab_closed
                && tab.view == view
                && (!matches!(view, View::Notes | View::References)
                    || tab.selected == self.workspace.selected)
        }) {
            self.workspace.select_tab(index);
            return true;
        }
        let tab = ViewTab {
            selected: self.workspace.selected.clone(),
            view,
            ..Default::default()
        };
        match self.workspace.open_tab(tab) {
            Ok(()) => true,
            Err(error) => {
                self.error = Some(error.to_string());
                false
            }
        }
    }

    pub(super) fn close_other_tabs(&mut self, index: usize, right_only: TabCloseScope) {
        let right_only = right_only == TabCloseScope::ToTheRight;

        self.focus_current_pane();
        let active = self.workspace.active;
        if right_only {
            self.workspace.tabs.truncate(index + 1);
            if active > index {
                self.workspace.active = index;
                self.reload_workbench_tab();
            }
        } else {
            let retained = self.workspace.tabs.remove(index);
            self.workspace.tabs = vec![retained];
            self.workspace.active = 0;
            if active != index {
                self.reload_workbench_tab();
            }
        }
    }

    fn reload_workbench_tab(&mut self) {
        let view = self.workspace.view;
        if view != View::Notes && !self.workspace.retain_output {
            self.load_view(view);
        }
    }
}

fn is_code_view(view: View) -> bool {
    matches!(
        view,
        View::Pseudocode | View::Assembly | View::Graph | View::Hex
    )
}
