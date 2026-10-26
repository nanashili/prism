use super::app::Prism;
use super::{icons::Icon, popup, tab_strip, widgets};
use crate::model::View;
use crate::ui::presentation::{Selection, TabCloseScope};
use eframe::egui::{self, Rect};

enum TabAction {
    Select(usize, bool),
    Close(usize),
    Keep(usize),
    CloseOthers(usize, bool),
}

impl Prism {
    pub(super) fn function_tab(&mut self, ui: &mut egui::Ui) {
        let controls = if self.workspace.split { 66.0 } else { 36.0 };
        let mut action = None;
        let reveal = self.workspace.reveal_active_tab;
        let controls = tab_strip::show(ui, controls, |ui| {
            for index in 0..self.workspace.tabs.len() {
                if !self.workspace.tabs[index].tab_closed {
                    let identity = self.workspace.tabs[index].identity;
                    ui.push_id(("tab", identity), |ui| {
                        self.workbench_tab_item(ui, index, reveal, &mut action);
                    });
                }
            }
        });
        self.workspace.reveal_active_tab = false;
        if let Some(action) = action {
            match action {
                TabAction::Select(index, pin) => {
                    self.select_workbench_tab(index);
                    if pin {
                        self.pin_workbench_tab(index);
                    }
                }
                TabAction::Close(index) => self.close_workbench_tab(index),
                TabAction::Keep(index) => self.pin_workbench_tab(index),
                TabAction::CloseOthers(index, right) => {
                    self.close_other_tabs(index, TabCloseScope::from(right))
                }
            }
        }
        self.pane_tab_controls(ui, controls);
    }

    fn pane_tab_controls(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let start = rect.left_top() + egui::vec2(4.0, 5.0);
        let mut buttons = ui.new_child(
            egui::UiBuilder::new().max_rect(Rect::from_min_size(start, egui::vec2(62.0, 26.0))),
        );
        buttons.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.add_enabled_ui(!self.workspace.split, |ui| {
                if widgets::icon_button(ui, Icon::Split, "Split right", 26.0).clicked() {
                    self.toggle_workbench_split();
                }
            });
            if self.workspace.split
                && widgets::icon_button(ui, Icon::Close, "Close this split", 26.0).clicked()
            {
                self.close_workbench_pane();
            }
        });
    }

    fn workbench_tab_item(
        &self,
        ui: &mut egui::Ui,
        index: usize,
        reveal: bool,
        action: &mut Option<TabAction>,
    ) {
        let tab = &self.workspace.tabs[index];
        let active = self.workspace.active == index;
        let focused = self
            .workspace
            .rendering_pane
            .unwrap_or(self.workspace.focused_pane)
            == self.workspace.focused_pane;
        let title = self.tab_title(index);
        let item = tab_strip::tab(
            ui,
            tab_strip::Tab {
                title: &title,
                active,
                focused,
                preview: tab.preview,
                busy: false,
            },
            |ui, rect, color| view_icon(tab.view).paint(ui, rect, color),
        );
        let response = item.response;
        if active && reveal {
            response.scroll_to_me(Some(egui::Align::Center));
        }
        if response.clicked() {
            *action = Some(TabAction::Select(index, response.double_clicked()));
        }
        if item.close_requested {
            *action = Some(TabAction::Close(index));
        }
        response.context_menu(|ui| self.workbench_tab_menu(ui, index, action));
    }

    fn workbench_tab_menu(&self, ui: &mut egui::Ui, index: usize, action: &mut Option<TabAction>) {
        ui.set_width(220.0);
        for (label, icon, command) in [
            ("Keep open", Icon::Bookmark, TabAction::Keep(index)),
            ("Close tab", Icon::Close, TabAction::Close(index)),
            (
                "Close other tabs",
                Icon::Close,
                TabAction::CloseOthers(index, false),
            ),
            (
                "Close tabs to the right",
                Icon::Close,
                TabAction::CloseOthers(index, true),
            ),
        ] {
            if popup::row(ui, Some(icon), label, "", Selection::Unselected).clicked() {
                *action = Some(command);
                ui.close();
            }
        }
    }
}

pub(super) fn view_icon(view: View) -> Icon {
    match view {
        View::Pseudocode => Icon::Pseudocode,
        View::Assembly => Icon::Assembly,
        View::Graph => Icon::Graph,
        View::References => Icon::References,
        View::Hex => Icon::Hex,
        View::Strings => Icon::Strings,
        View::Imports => Icon::Imports,
        View::Segments => Icon::Segments,
        View::Resources => Icon::Source,
        View::Notes => Icon::Note,
        View::Diff => Icon::Split,
        View::Debug => Icon::Play,
        View::Reconstruct => Icon::Spark,
    }
}
