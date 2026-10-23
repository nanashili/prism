use super::app::{Chat, Prism};
use crate::error::Result;
use crate::model::Message;
use crate::storage::{MessagePage, MessageWindow, Store};
use eframe::egui;

#[derive(Default)]
pub(super) struct History {
    pub has_earlier: bool,
    pub page: Option<MessagePage>,
}

impl History {
    pub fn controls(&self, ui: &mut egui::Ui, latest: &[Message]) -> Option<MessageWindow> {
        let messages = self.page.as_ref().map_or(latest, |page| &page.messages);
        let earlier = self
            .page
            .as_ref()
            .map_or(self.has_earlier, |page| page.has_earlier);
        if !earlier && self.page.is_none() {
            return None;
        }

        let mut request = None;
        ui.horizontal_wrapped(|ui| {
            let first = messages.iter().find_map(|message| message.id);
            if history_button(
                ui,
                "Earlier",
                earlier && first.is_some(),
                "Load earlier messages",
            ) {
                request = first.map(MessageWindow::Before);
            }
            if let Some(page) = &self.page {
                let last = messages.iter().rev().find_map(|message| message.id);
                if history_button(
                    ui,
                    "Newer",
                    page.has_later && last.is_some(),
                    "Load newer messages",
                ) {
                    request = last.map(MessageWindow::After);
                }
                if history_button(ui, "Latest", true, "Return to the live conversation") {
                    request = Some(MessageWindow::Latest);
                }
            }
        });
        request
    }
}

fn history_button(ui: &mut egui::Ui, title: &str, enabled: bool, help: &str) -> bool {
    ui.add_enabled(enabled, egui::Button::new(title).small())
        .on_hover_text(help)
        .clicked()
}

impl Chat {
    pub(super) fn restore(store: &Store, id: i64) -> Result<Self> {
        let page = store.message_page(id, MessageWindow::Latest)?;
        let (draft, attachments) = store.chat_draft(id)?;
        let goal = page
            .messages
            .iter()
            .rev()
            .find_map(|message| {
                message
                    .activity
                    .as_ref()
                    .map(|activity| activity.progress.goal.clone())
            })
            .flatten();
        Ok(Self {
            id: Some(id),
            title: store.chat_title(id)?,
            messages: page.messages,
            history: History {
                has_earlier: page.has_earlier,
                ..Default::default()
            },
            draft,
            attachments,
            goal,
            queue: store.chat_queue(id)?,
            models: store.chat_models(id)?,
            ..Default::default()
        })
    }
}

impl Prism {
    pub(super) fn navigate_chat_history(&mut self, window: MessageWindow) {
        if matches!(window, MessageWindow::Latest) {
            self.chat.history.page = None;
            return;
        }
        let Some(id) = self.chat.id else {
            return;
        };
        match self.store.message_page(id, window) {
            Ok(page) if !page.messages.is_empty() => self.chat.history.page = Some(page),
            Ok(_) => match (&mut self.chat.history.page, window) {
                (Some(page), MessageWindow::Before(_)) => page.has_earlier = false,
                (Some(page), MessageWindow::After(_)) => page.has_later = false,
                _ => self.chat.history.has_earlier = false,
            },
            Err(error) => self.report_error(error),
        }
    }
}
