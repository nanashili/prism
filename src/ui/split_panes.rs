use super::{app::Prism, theme::Palette, view_pane::ViewPane};
use crate::model::View;
use eframe::egui;

impl Prism {
    pub(super) fn workbench_panes(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let (panes, count) = if self.workspace.split {
            let middle = self.workbench_divider(ui, rect);
            (
                [
                    egui::Rect::from_min_max(rect.min, egui::pos2(middle - 3.0, rect.bottom())),
                    egui::Rect::from_min_max(egui::pos2(middle + 3.0, rect.top()), rect.max),
                ],
                2,
            )
        } else {
            ([rect, egui::Rect::NOTHING], 1)
        };
        for (index, pane) in panes.into_iter().take(count).enumerate() {
            if index > 0 && !self.workspace.split {
                break;
            }
            if ui.rect_contains_pointer(pane) && ui.input(|input| input.pointer.any_pressed()) {
                self.workspace.focused_pane = index;
            }
            self.workspace.rendering_pane = Some(index);
            let identity = self.workspace.identity;
            let mut pane_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("workbench-pane", identity))
                    .max_rect(pane),
            );
            self.workbench_pane(&mut pane_ui);
        }
        self.workspace.rendering_pane = None;
    }

    fn workbench_divider(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> f32 {
        let fraction = self.workspace.split_fraction.unwrap_or(0.5);
        let mut middle = rect.left() + rect.width() * fraction;
        let handle = egui::Rect::from_min_max(
            egui::pos2(middle - 3.0, rect.top()),
            egui::pos2(middle + 3.0, rect.bottom()),
        );
        let response = ui
            .interact(handle, ui.id().with("pane-divider"), egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
            && rect.width() > 0.0
        {
            let minimum = (180.0 / rect.width()).min(0.5);
            let fraction = ((pointer.x - rect.left()) / rect.width()).clamp(minimum, 1.0 - minimum);
            self.workspace.split_fraction = Some(fraction);
            middle = rect.left() + rect.width() * fraction;
        }
        ui.painter().line_segment(
            [
                egui::pos2(middle, rect.top()),
                egui::pos2(middle, rect.bottom()),
            ],
            egui::Stroke::new(1.0, Palette::for_context(ui.ctx()).fill),
        );
        middle
    }

    pub(super) fn toggle_workbench_split(&mut self) {
        if self.workspace.split {
            self.retain_workbench_pane(self.current_pane());
        } else {
            let selection = self.workspace.duplicate_selection();
            let view = selection.view;
            self.install_split_pane(1, selection);
            self.select_view(view);
        }
    }

    pub(super) fn open_function_in_split(&mut self, address: &str, name: &str, view: View) {
        let destination = 1 - self.current_pane();
        if self.workspace.split {
            self.focus_workbench_pane(destination);
        } else {
            self.install_split_pane(destination, Default::default());
        }
        self.open_function_view(address, name, view);
    }

    pub(super) fn close_workbench_pane(&mut self) {
        if self.workspace.split {
            self.retain_workbench_pane(1 - self.current_pane());
        }
    }

    pub(super) fn focus_current_pane(&mut self) {
        self.workspace.focused_pane = self.current_pane();
    }

    fn current_pane(&self) -> usize {
        self.workspace
            .rendering_pane
            .unwrap_or(self.workspace.focused_pane)
    }

    fn focus_workbench_pane(&mut self, index: usize) {
        self.workspace.focused_pane = index;
        if self.workspace.rendering_pane.is_some() {
            self.workspace.rendering_pane = Some(index);
        }
    }

    fn retain_workbench_pane(&mut self, index: usize) {
        let pane = std::mem::take(&mut self.workspace.panes[index]);
        self.workspace.panes = [pane, Default::default()];
        self.workspace.split = false;
        self.focus_workbench_pane(0);
    }

    fn install_split_pane(&mut self, index: usize, mut pane: ViewPane) {
        self.workspace.next_pane_identity = self.workspace.next_pane_identity.wrapping_add(1);
        pane.identity = self.workspace.next_pane_identity;
        self.workspace.panes[index] = pane;
        self.workspace.split = true;
        self.focus_workbench_pane(index);
    }
}
