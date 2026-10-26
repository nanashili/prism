use crate::{
    model::ChatSummary,
    ui::{app::Prism, icons::Icon, popup, theme::Palette, widgets},
};
use eframe::egui::{self, FontId, Rect, Sense};

#[derive(Default)]
pub(in crate::ui) struct History {
    query: String,
    rows: Vec<ChatSummary>,
    offset: usize,
    more: bool,
    armed: Option<i64>,
}

impl Prism {
    pub(in crate::ui) fn past_chats_button(&mut self, ui: &mut egui::Ui) {
        let response = widgets::icon_button(ui, Icon::History, "Past chats", 28.0);
        if response.clicked() {
            self.chat_tabs.history.offset = 0;
            self.chat_tabs.history.armed = None;
            self.refresh_past_chats();
        }
        popup::menu(&response, 320.0)
            .align(egui::RectAlign::BOTTOM_END)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                let name = self
                    .workspace
                    .project
                    .as_ref()
                    .map_or("this project", |project| project.target.name.as_str());
                let search = popup::search(
                    ui,
                    &mut self.chat_tabs.history.query,
                    format!("Search chats in {name}"),
                );
                if response.clicked() {
                    search.request_focus();
                }
                if search.changed() {
                    self.chat_tabs.history.offset = 0;
                    self.chat_tabs.history.armed = None;
                    self.refresh_past_chats();
                }
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .id_salt("past-chats")
                    .max_height(360.0)
                    .show(ui, |ui| self.past_chat_rows(ui));
                self.past_chat_paging(ui);
            });
    }

    fn refresh_past_chats(&mut self) {
        let project = self
            .workspace
            .project
            .as_ref()
            .map(|project| project.target.sha256.as_str());
        match self.store.past_chats(
            project,
            &self.chat_tabs.history.query,
            self.chat_tabs.history.offset,
        ) {
            Ok(mut rows) => {
                self.chat_tabs.history.more = rows.len() > 50;
                rows.truncate(50);
                self.chat_tabs.history.rows = rows;
            }
            Err(error) => {
                self.chat_tabs.history.rows.clear();
                self.report_error(error);
            }
        }
    }

    fn past_chat_rows(&mut self, ui: &mut egui::Ui) {
        for index in 0..self.chat_tabs.history.rows.len() {
            let chat = &self.chat_tabs.history.rows[index];
            let id = chat.id;
            let title = chat.title.clone();
            let (open, delete) = past_chat(ui, chat, self.chat_tabs.history.armed == Some(id));
            if !open.hovered() && !delete.hovered() && self.chat_tabs.history.armed == Some(id) {
                self.chat_tabs.history.armed = None;
            }
            if open.clicked() {
                self.load_chat(id);
                ui.close();
                break;
            }
            if delete.clicked() {
                if self.chat_tabs.history.armed == Some(id) {
                    self.delete_chats(&[id]);
                    self.chat_tabs.history.armed = None;
                    self.refresh_past_chats();
                    break;
                }
                self.chat_tabs.history.armed = Some(id);
            }
            open.context_menu(|ui| self.chat_menu(ui, id, &title));
        }
        if self.chat_tabs.history.rows.is_empty() {
            ui.add_space(8.0);
            let text = if self.chat_tabs.history.query.trim().is_empty() {
                "No chats on this file yet."
            } else {
                "No chats match that."
            };
            ui.colored_label(Palette::for_context(ui.ctx()).muted, text);
            ui.add_space(8.0);
        }
    }

    fn past_chat_paging(&mut self, ui: &mut egui::Ui) {
        let history = &mut self.chat_tabs.history;
        if history.offset == 0 && !history.more {
            return;
        }
        let mut changed = false;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(history.offset > 0, egui::Button::new("Newer"))
                .clicked()
            {
                history.offset = history.offset.saturating_sub(50);
                changed = true;
            }
            if ui
                .add_enabled(history.more, egui::Button::new("Older"))
                .clicked()
            {
                history.offset += 50;
                changed = true;
            }
        });
        if changed {
            self.refresh_past_chats();
        }
    }
}

fn past_chat(
    ui: &mut egui::Ui,
    chat: &ChatSummary,
    armed: bool,
) -> (egui::Response, egui::Response) {
    let p = Palette::for_context(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::hover());
    let delete_rect = Rect::from_min_max(
        egui::pos2(rect.right() - if armed { 64.0 } else { 28.0 }, rect.top()),
        rect.max,
    );
    let open = ui.interact(
        Rect::from_min_max(rect.min, egui::pos2(delete_rect.left(), rect.bottom())),
        ui.id().with((chat.id, "past-open")),
        Sense::click(),
    );
    let delete = ui.interact(
        delete_rect,
        ui.id().with((chat.id, "past-delete")),
        Sense::click(),
    );
    let hovered = open.hovered() || delete.hovered();
    if hovered {
        ui.painter().rect_filled(rect, 7.0, p.fill);
    }
    let minutes = (crate::model::now_ms() / 1000 - chat.updated).max(0) / 60;
    let when = match minutes {
        0 => "now".into(),
        1..60 => format!("{minutes}m"),
        60..1440 => format!("{}h", minutes / 60),
        _ => format!("{}d", minutes / 1440),
    };
    let mut text = egui::text::LayoutJob::simple_singleline(
        chat.title.clone(),
        FontId::proportional(12.5),
        p.text3,
    );
    text.wrap.max_width = (rect.width()
        - if hovered {
            delete_rect.width() + 16.0
        } else {
            54.0
        })
    .max(0.0);
    text.wrap.max_rows = 1;
    let text = ui.fonts_mut(|fonts| fonts.layout_job(text));
    ui.painter().galley(
        rect.left_center() + egui::vec2(8.0, -text.size().y / 2.0),
        text,
        p.text3,
    );
    if hovered {
        if armed {
            ui.painter().text(
                delete_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Delete?",
                FontId::proportional(12.0),
                ui.visuals().error_fg_color,
            );
        } else {
            Icon::Trash.paint(
                ui,
                Rect::from_center_size(delete_rect.center(), egui::vec2(13.0, 13.0)),
                p.faint,
            );
        }
    } else {
        ui.painter().text(
            rect.right_center() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            when,
            FontId::proportional(11.5),
            p.faint,
        );
    }
    open.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &chat.title));
    delete.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            if armed {
                "Confirm delete chat"
            } else {
                "Delete chat"
            },
        )
    });
    (open, delete.on_hover_text("Delete chat"))
}
