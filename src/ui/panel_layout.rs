use super::{app::Prism, theme::Palette};
use eframe::egui::{self, Rect, Sense, UiBuilder};

pub(super) fn expansion(context: &egui::Context, name: &str, expanded: bool) -> f32 {
    let progress = context.animate_bool_with_time(egui::Id::new(name), expanded, 0.24);
    progress * progress * (3.0 - 2.0 * progress)
}

impl Prism {
    pub(super) fn workbench_columns(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let expanded = expansion(
            ui.ctx(),
            "conversation-expansion",
            !self.workspace.agent_closed,
        );
        let maximum = (rect.width() - 240.0).clamp(0.0, 760.0);
        let width = conversation_width(ui, self.settings.desktop.chat_panel_width, maximum);
        let divider = rect.right() - width * expanded;
        let code = Rect::from_min_max(rect.min, egui::pos2(divider, rect.bottom()));

        ui.allocate_rect(rect, Sense::hover());
        let mut code_ui = clipped_column(ui, "workbench-code", code);
        self.workbench_panes(&mut code_ui);

        if expanded > 0.0 {
            let visible = Rect::from_min_max(egui::pos2(divider, rect.top()), rect.max);
            let content = Rect::from_min_size(visible.min, egui::vec2(width, rect.height()));
            let mut chat_ui = clipped_column(ui, "workbench-conversation", content);
            chat_ui.set_clip_rect(chat_ui.clip_rect().intersect(visible));
            if expanded < 1.0 {
                chat_ui.disable();
            }
            self.agent_dock(&mut chat_ui);
        }

        if expanded == 1.0 {
            self.conversation_divider(ui, rect, divider, maximum);
        }
    }

    fn conversation_divider(&mut self, ui: &mut egui::Ui, rect: Rect, divider: f32, maximum: f32) {
        let handle = Rect::from_x_y_ranges(divider - 3.0..=divider + 3.0, rect.y_range());
        let response = ui
            .interact(handle, ui.id().with("conversation-divider"), Sense::drag())
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);

        if maximum >= 400.0
            && response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let width = (rect.right() - pointer.x).clamp(400.0_f32.min(maximum), maximum);
            if self.settings.desktop.chat_panel_width != width {
                self.settings.desktop.chat_panel_width = width;
                self.queue_settings_save(ui.ctx());
                ui.ctx().request_repaint();
            }
        }

        let palette = Palette::for_context(ui.ctx());
        let stroke = if response.hovered() || response.dragged() {
            palette.accent
        } else {
            palette.fill
        };
        ui.painter().vline(divider, rect.y_range(), (1.0, stroke));
    }
}

fn conversation_width(ui: &egui::Ui, saved_width: f32, maximum: f32) -> f32 {
    let target = saved_width.min(maximum);
    let duration = ui.style().animation_time.clamp(0.0, 0.12);
    let animated =
        ui.ctx()
            .animate_value_with_time(ui.id().with("conversation-width"), target, duration);

    if duration == 0.0 {
        target
    } else {
        animated.min(maximum)
    }
}

fn clipped_column(ui: &mut egui::Ui, name: &str, rect: Rect) -> egui::Ui {
    let mut column = ui.new_child(UiBuilder::new().id_salt(name).max_rect(rect));
    column.set_clip_rect(ui.clip_rect().intersect(rect));
    column
}
