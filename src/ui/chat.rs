mod choices;
mod goal;
mod permission;
pub(in crate::ui) mod questions;
mod queue;
pub(in crate::ui) mod transcript;
mod updates;

use super::app::Prism;
use eframe::egui::{self, Frame};

impl Prism {
    pub(super) fn conversation(&mut self, ui: &mut egui::Ui) {
        self.conversation_composer(ui);
        if self.chat.messages.is_empty() && self.screen == crate::model::Screen::Workbench {
            self.transcript.clear_bodies();
            self.empty_agent(ui);
            return;
        }

        let navigation = self.chat.history.controls(ui, &self.chat.messages);
        self.conversation_messages(ui);
        if let Some(window) = navigation {
            self.navigate_chat_history(window);
        }
    }

    fn conversation_composer(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::bottom(ui.id().with("chat-composer"))
            .show_separator_line(false)
            .frame(Frame::NONE.inner_margin(egui::Margin {
                left: 10,
                right: 10,
                top: 8,
                bottom: 10,
            }))
            .show_inside(ui, |ui| {
                self.permission_card(ui);
                self.queued_follow_ups(ui);
                self.goal_bar(ui);
                self.question_card(ui);
                ui.add_space(8.0);
                self.composer(ui);
            });
    }

    fn conversation_messages(&mut self, ui: &mut egui::Ui) {
        let style = super::message::style::ChatStyle::new(ui.ctx(), &self.settings);
        self.chat.transcript_ids.ensure(self.chat.messages.len());
        self.transcript.show(
            ui,
            &self.chat,
            self.chat_tabs.active,
            self.settings.conversation.show_thinking,
            &style,
        );
    }
}
