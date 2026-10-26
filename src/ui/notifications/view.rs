use super::{card, state::Center};
use crate::ui::presentation::{ChatNavigation, NoticeContext};
use crate::ui::{icons::Icon, popup, theme::Palette, widgets};
use eframe::egui::{self, Align2, FontId, Rect, Sense, Stroke};

impl Center {
    pub(in crate::ui) fn button(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::click());
        if response.hovered() || self.open {
            ui.painter().rect_filled(rect, 6.0, p.fill);
        }
        bell(ui, rect, p.muted);
        let unread = self.unread();
        if unread > 0 {
            let badge = Rect::from_center_size(
                rect.right_top() + egui::vec2(-7.0, 8.0),
                egui::vec2(15.0, 13.0),
            );
            ui.painter().rect_filled(badge, 6.0, p.accent);
            ui.painter().text(
                badge.center(),
                Align2::CENTER_CENTER,
                unread.to_string(),
                FontId::proportional(9.0),
                p.card,
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Notifications")
        });
        if response
            .on_hover_text(format!("Notifications · {unread} unread"))
            .clicked()
        {
            self.open = !self.open;
            if self.open {
                self.mark_read();
            }
        }
    }

    pub(super) fn show(&mut self, context: &egui::Context) -> Option<i64> {
        if self.open
            && context
                .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.open = false;
        }
        let width = (context.content_rect().width() - 32.0).clamp(180.0, 392.0);
        let mut open_chat = None;
        let area = egui::Area::new("notifications".into())
            .anchor(Align2::RIGHT_BOTTOM, [-16.0, -46.0])
            .order(egui::Order::Foreground)
            .show(context, |ui| {
                ui.set_width(width);
                ui.set_max_height((context.content_rect().height() - 110.0).max(100.0));
                if self.open {
                    open_chat = self.history(ui);
                } else {
                    open_chat = self.toasts(ui);
                }
                ui.min_rect()
            });
        if area.inner.min.distance(area.response.rect.min) > 1.0 {
            context.request_discard("notification panel resized");
        }
        if self.expire() {
            context.request_repaint();
        }
        open_chat
    }

    fn toasts(&mut self, ui: &mut egui::Ui) -> Option<i64> {
        let height = (ui.ctx().content_rect().height() - 110.0).max(100.0);
        let mut open_chat = None;
        egui::ScrollArea::vertical()
            .id_salt("notification-stack")
            .max_height(height)
            .show(ui, |ui| {
                for entry in self.entries.iter_mut().filter(|entry| entry.toast).take(3) {
                    ui.push_id(entry.id, |ui| {
                        if card::show(
                            ui,
                            entry,
                            ChatNavigation::from(entry.chat.is_some()),
                            NoticeContext::Toast,
                        ) {
                            open_chat = entry.chat;
                        }
                    });
                    ui.add_space(8.0);
                }
            });
        open_chat
    }

    fn history(&mut self, ui: &mut egui::Ui) -> Option<i64> {
        let mut open_chat = None;
        popup::surface(ui.ctx(), 10).show(ui, |ui| {
            ui.horizontal(|ui| {
                let heading_width = (ui.available_width() - 95.0).max(30.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(heading_width, 26.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_size(egui::vec2(heading_width, 26.0));
                        ui.label(egui::RichText::new("Notifications").strong());
                    },
                );
                if ui
                    .add_enabled(
                        !self.entries.is_empty(),
                        egui::Button::new("Clear all").small(),
                    )
                    .clicked()
                {
                    self.entries.clear();
                }
                if widgets::icon_button(ui, Icon::Close, "Close notifications", 22.0).clicked() {
                    self.open = false;
                }
            });
            ui.add_space(6.0);
            if self.entries.is_empty() {
                ui.label(egui::RichText::new("You're all caught up").size(13.0));
                ui.label(
                    egui::RichText::new("Recent notifications appear here while Prism is open.")
                        .size(12.0)
                        .weak(),
                );
                ui.add_space(8.0);
            }
            let height = (ui.ctx().content_rect().height() * 0.65).max(100.0);
            egui::ScrollArea::vertical()
                .id_salt("notification-history")
                .max_height(height)
                .show(ui, |ui| {
                    for entry in &mut self.entries {
                        ui.push_id((entry.id, "history"), |ui| {
                            if card::show(
                                ui,
                                entry,
                                ChatNavigation::from(entry.chat.is_some()),
                                NoticeContext::History,
                            ) {
                                open_chat = entry.chat;
                            }
                        });
                        ui.add_space(8.0);
                    }
                });
        });
        open_chat
    }
}

fn bell(ui: &egui::Ui, rect: Rect, color: egui::Color32) {
    let center = rect.center();
    let points = [
        (-7.0, 5.0),
        (-5.0, 2.0),
        (-5.0, -3.0),
        (-3.0, -6.0),
        (3.0, -6.0),
        (5.0, -3.0),
        (5.0, 2.0),
        (7.0, 5.0),
        (-7.0, 5.0),
    ]
    .into_iter()
    .map(|(x, y)| center + egui::vec2(x, y))
    .collect();
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.3, color)));
    ui.painter().line_segment(
        [
            center + egui::vec2(0.0, -8.0),
            center + egui::vec2(0.0, -6.0),
        ],
        Stroke::new(1.3, color),
    );
    ui.painter().line_segment(
        [
            center + egui::vec2(-2.0, 8.0),
            center + egui::vec2(2.0, 8.0),
        ],
        Stroke::new(1.5, color),
    );
}
