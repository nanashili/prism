use crate::error::Result;
use crate::model::QueuedPrompt;
use crate::ui::{app::Prism, icons::Icon, widgets};
use eframe::egui;

impl Prism {
    pub(in crate::ui) fn enqueue_follow_up(&mut self) -> Result<()> {
        if self.chat.queue.len() >= QueuedPrompt::LIMIT {
            return Err("Eight follow-ups are queued. Send or remove one first.".into());
        }
        let chat = self
            .chat
            .id
            .ok_or("Start a chat before queuing a follow-up")?;
        self.chat.queue.push_back(QueuedPrompt {
            text: crate::ui::composer::mentions::with_mentions(
                &self.chat.draft,
                &self.chat.mentions,
            ),
            attachments: self.chat.attachments.clone(),
        });
        if let Err(error) = self
            .store
            .save_queue_and_draft(chat, &self.chat.queue, "", &[])
        {
            self.chat.queue.pop_back();
            return Err(error);
        }
        self.chat.draft.clear();
        self.chat.attachments.clear();
        self.chat.mentions.clear();
        Ok(())
    }

    pub(in crate::ui) fn send_next_follow_up(&mut self) {
        if self.chat.busy {
            return;
        }
        let Some(item) = self.chat.queue.pop_front() else {
            return;
        };
        let draft = std::mem::replace(&mut self.chat.draft, item.text);
        let attachments = std::mem::replace(&mut self.chat.attachments, item.attachments);
        let result = self.prepare_prompt();
        let unsent = QueuedPrompt {
            text: std::mem::replace(&mut self.chat.draft, draft),
            attachments: std::mem::replace(&mut self.chat.attachments, attachments),
        };
        if let Err(error) = result {
            if !self.chat.busy {
                self.chat.queue.push_front(unsent);
            }
            self.report_error(error);
        }
        if let Some(chat) = self.chat.id
            && let Err(error) = self.store.save_chat_queue(chat, &self.chat.queue)
        {
            self.report_error(error);
        }
        if let Err(error) = self.save_chat_draft() {
            self.report_error(error);
        }
    }

    pub(in crate::ui) fn queued_follow_ups(&mut self, ui: &mut egui::Ui) {
        if self.chat.queue.is_empty() {
            return;
        }
        ui.horizontal(|ui| {
            ui.weak(format!("Queued follow-ups · {}", self.chat.queue.len()));
            if !self.chat.busy && ui.small_button("Send next").clicked() {
                self.send_next_follow_up();
            }
        });
        let mut action = None;
        egui::ScrollArea::vertical()
            .max_height(112.0)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                for (index, item) in self.chat.queue.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            let title = if item.text.trim().is_empty() {
                                item.attachments
                                    .first()
                                    .map_or("Attachments", |item| &item.name)
                                    .into()
                            } else {
                                crate::ui::playbooks::shown_text(&item.text)
                            };
                            ui.add_sized(
                                [ui.available_width() - 48.0, 24.0],
                                egui::Label::new(title).truncate(),
                            );
                            let editable =
                                self.chat.draft.is_empty() && self.chat.attachments.is_empty();
                            ui.add_enabled_ui(editable, |ui| {
                                if widgets::icon_button(ui, Icon::Rename, "Move to draft", 20.0)
                                    .clicked()
                                {
                                    action = Some((index, true));
                                }
                            });
                            if widgets::icon_button(ui, Icon::Close, "Remove follow-up", 20.0)
                                .clicked()
                            {
                                action = Some((index, false));
                            }
                        })
                    });
                }
            });
        if let Some((index, edit)) = action
            && let Err(error) = self.remove_follow_up(index, edit)
        {
            self.report_error(error);
        }
    }

    fn remove_follow_up(&mut self, index: usize, edit: bool) -> Result<()> {
        let chat = self.chat.id.ok_or("This chat has not been saved")?;
        let Some(item) = self.chat.queue.remove(index) else {
            return Ok(());
        };
        let (draft, attachments) = if edit {
            (item.text.as_str(), item.attachments.as_slice())
        } else {
            (self.chat.draft.as_str(), self.chat.attachments.as_slice())
        };
        if let Err(error) =
            self.store
                .save_queue_and_draft(chat, &self.chat.queue, draft, attachments)
        {
            self.chat.queue.insert(index, item);
            return Err(error);
        }
        if edit {
            self.chat.draft = item.text;
            self.chat.attachments = item.attachments;
        }
        Ok(())
    }
}
