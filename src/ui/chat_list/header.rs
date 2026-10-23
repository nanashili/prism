use crate::ui::{app::Prism, icons::Icon, popup, theme::Palette, widgets};
use eframe::egui::{self, Align2, FontId, Rect, Sense};

impl Prism {
    pub(super) fn chat_list_header(
        &mut self,
        ui: &mut egui::Ui,
        collapsible: bool,
    ) -> (bool, bool) {
        let mut expanded = !collapsible || self.sidebar_section_open("chats");
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let title = if self.chat_list.archived {
                "Archived chats"
            } else {
                "Chats"
            };
            let title = heading(ui, title, collapsible, expanded).on_hover_ui(|ui| {
                ui.label(format!("Chats: {}", self.chat_list.total));
            });
            if title.clicked() && collapsible {
                expanded = !expanded;
                self.set_sidebar_section_open("chats", expanded);
            }

            if widgets::icon_button(ui, Icon::Search, "Search chats", 24.0).clicked() {
                self.chat_list.search = !self.chat_list.search;
                if !self.chat_list.search {
                    self.chat_list.filter.clear();
                    changed = true;
                }
                expanded = true;
            }
            if self.chat_list_options(ui) {
                changed = true;
                expanded = true;
            }
            let new = widgets::icon_button(ui, Icon::New, "New chat", 24.0);
            if new.clicked() {
                self.new_chat();
                self.workspace.agent_closed = false;
                changed = true;
                expanded = true;
            }
        });
        if collapsible && expanded && !self.sidebar_section_open("chats") {
            self.set_sidebar_section_open("chats", true);
        }
        (expanded, changed)
    }

    fn chat_list_options(&mut self, ui: &mut egui::Ui) -> bool {
        let more = widgets::icon_button(ui, Icon::More, "Chat options", 24.0);
        let mut changed = false;
        popup::menu(&more, 210.0).show(|ui| {
            if popup::row(
                ui,
                Some(Icon::Folder),
                "Archived chats",
                "",
                self.chat_list.archived,
            )
            .clicked()
            {
                self.chat_list.archived = !self.chat_list.archived;
                changed = true;
                ui.close();
            }
        });
        changed
    }
}

fn heading(ui: &mut egui::Ui, title: &str, collapsible: bool, expanded: bool) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let width = (ui.available_width() - 78.0).max(0.0);
    let sense = if collapsible {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), sense);
    if collapsible && response.hovered() {
        ui.painter().rect_filled(rect, 6.0, p.fill);
    }
    let text = ui.painter().text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(12.0),
        p.muted,
    );
    if collapsible {
        let icon = if expanded {
            Icon::Disclosure
        } else {
            Icon::Forward
        };
        icon.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(text.right() + 10.0, rect.center().y),
                egui::vec2(10.0, 10.0),
            ),
            p.faint,
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::CollapsingHeader,
                ui.is_enabled(),
                expanded,
                title,
            )
        });
    }
    response
}
