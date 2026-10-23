use super::{app::Prism, icons::Icon, popup};
use eframe::egui;

impl Prism {
    pub(super) fn chat_menu(&mut self, ui: &mut egui::Ui, id: i64, title: &str) {
        if popup::row(ui, Some(Icon::Thread), "Open", "", false).clicked() {
            self.load_chat(id);
            ui.close();
        }
        if popup::row(ui, Some(Icon::Rename), "Rename", "", false).clicked() {
            self.chat_rename = Some((id, title.into()));
            ui.close();
        }
        if let Some(chat) = self.chats.iter().find(|chat| chat.id == id) {
            let pinned = chat.pinned;
            let archived = chat.archived;
            if popup::row(
                ui,
                Some(Icon::Star),
                if pinned { "Unpin" } else { "Pin" },
                "",
                false,
            )
            .clicked()
            {
                let result = self
                    .store
                    .pin_chat(id, !pinned)
                    .and_then(|()| self.refresh_chats());
                if let Err(error) = result {
                    self.error = Some(error.to_string());
                }
                ui.close();
            }
            if popup::row(
                ui,
                Some(Icon::Folder),
                if archived { "Unarchive" } else { "Archive" },
                "",
                false,
            )
            .clicked()
            {
                let result = self
                    .store
                    .archive_chat(id, !archived)
                    .and_then(|()| self.refresh_chats());
                let open = if self.chat.id == Some(id) {
                    Some(self.chat_tabs.active)
                } else {
                    self.chat_tabs
                        .parked
                        .iter()
                        .find(|tab| tab.chat.id == Some(id))
                        .map(|tab| tab.key)
                };
                match (result, open) {
                    (Ok(()), Some(key)) if !archived => self.close_chat_tab(key),
                    (Err(error), _) => self.error = Some(error.to_string()),
                    _ => {}
                }
                ui.close();
            }
        }
        ui.separator();
        if popup::row(ui, Some(Icon::New), "New chat", "", false).clicked() {
            self.new_chat();
            ui.close();
        }
        ui.separator();
        popup::submenu(ui, Icon::Close, "Delete", |ui| {
            if popup::row(
                ui,
                Some(Icon::Close),
                &format!("Delete \"{title}\" for good"),
                "",
                false,
            )
            .clicked()
            {
                self.delete_chats(&[id]);
                ui.close();
            }
        });
    }

    pub(super) fn delete_chats(&mut self, ids: &[i64]) {
        let result = (|| -> crate::error::Result<()> {
            let mut keys: Vec<_> = self
                .chat_tabs
                .parked
                .iter()
                .filter(|tab| tab.chat.id.is_some_and(|id| ids.contains(&id)))
                .map(|tab| tab.key)
                .collect();
            if self.chat.id.is_some_and(|id| ids.contains(&id)) {
                keys.push(self.chat_tabs.active);
            }
            for key in keys {
                self.close_open_chat(key)?;
            }
            self.store.delete_chats(ids)?;
            self.refresh_chats()
        })();
        match result {
            Ok(()) => match self.store.recent(self.settings.recent_limit) {
                Ok(recent) => self.recent = recent,
                Err(error) => self.error = Some(error.to_string()),
            },
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(super) fn rename_chat_dialog(&mut self, context: &egui::Context) {
        let Some((id, mut title)) = self.chat_rename.take() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        egui::Window::new("Rename chat")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(context, |ui| {
                let input = ui.add(egui::TextEdit::singleline(&mut title).char_limit(160));
                if !context.memory(|memory| memory.focused().is_some()) {
                    input.request_focus();
                }
                save = ui
                    .add_enabled(!title.trim().is_empty(), egui::Button::new("Save"))
                    .clicked()
                    || (input.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            });
        if save && !title.trim().is_empty() {
            match self
                .store
                .rename_chat(id, title.trim())
                .and_then(|()| self.refresh_chats())
            {
                Ok(()) => {
                    if self.chat.id == Some(id) {
                        self.chat.title = title.trim().into();
                    }
                    for tab in &mut self.chat_tabs.parked {
                        if tab.chat.id == Some(id) {
                            tab.chat.title = title.trim().into();
                        }
                    }
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.chat_rename = Some((id, title));
                }
            }
        } else if open {
            self.chat_rename = Some((id, title));
        }
    }
}
