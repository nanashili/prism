use crate::storage::ChatArchive;
use crate::ui::presentation::{ActivityState, HeaderBehavior, Selection};
mod header;

use super::{app::Prism, sidebar_rows::chat_row, theme::Palette};
use eframe::egui::{self, RichText};

#[derive(Default)]
pub(super) struct ChatList {
    project: Option<String>,
    filter: String,
    search: bool,
    archived: bool,
    offset: usize,
    has_more: bool,
    ready: bool,
    total: usize,
}

impl Prism {
    pub(super) fn refresh_chats(&mut self) -> crate::error::Result<()> {
        let project = self
            .workspace
            .project
            .as_ref()
            .map(|project| project.target.sha256.clone());
        if self.chat_list.project != project {
            self.chat_list = ChatList {
                project: project.clone(),
                ..Default::default()
            };
        }
        let mut chats = self.store.chats_for(
            project.as_deref(),
            &self.chat_list.filter,
            ChatArchive::from(self.chat_list.archived),
            self.chat_list.offset,
        )?;
        self.chat_list.total = self.store.chat_count(
            project.as_deref(),
            ChatArchive::from(self.chat_list.archived),
        )?;
        self.chat_list.has_more = chats.len() > 50;
        chats.truncate(50);
        self.chats = chats;
        self.chat_list.ready = true;
        Ok(())
    }

    pub(super) fn sidebar_chats(&mut self, ui: &mut egui::Ui) {
        self.chat_list_section(ui, false);
    }

    pub(super) fn workbench_chats(&mut self, ui: &mut egui::Ui) {
        self.chat_list_section(ui, true);
    }

    fn chat_list_section(&mut self, ui: &mut egui::Ui, collapsible: bool) {
        let project = self
            .workspace
            .project
            .as_ref()
            .map(|project| project.target.sha256.as_str());
        if (!self.chat_list.ready || self.chat_list.project.as_deref() != project)
            && let Err(error) = self.refresh_chats()
        {
            ui.label("Couldn't load chats");
            self.error = Some(error.to_string());
            return;
        }
        if self.screen != crate::model::Screen::Workbench {
            ui.add_space(16.0);
        }
        let was_searching = self.chat_list.search;
        let (expanded, mut changed) = self.chat_list_header(ui, HeaderBehavior::from(collapsible));
        if !expanded {
            return;
        }
        if self.chat_list.search {
            let search =
                super::popup::search(ui, &mut self.chat_list.filter, "Search chats".into());
            if !was_searching {
                search.request_focus();
            }
            changed |= search.changed();
            ui.add_space(6.0);
        }
        if changed {
            self.chat_list.offset = 0;
            if let Err(error) = self.refresh_chats() {
                self.error = Some(error.to_string());
            }
        }
        if self.screen == crate::model::Screen::Workbench {
            egui::ScrollArea::vertical()
                .id_salt("project-chat-list")
                .auto_shrink([false, true])
                .max_height(166.0)
                .show(ui, |ui| self.chat_list_rows(ui));
        } else {
            self.chat_list_rows(ui);
        }
        self.chat_list_paging(ui);
    }

    fn chat_list_rows(&mut self, ui: &mut egui::Ui) {
        let mut pinned = None;
        for index in 0..self.chats.len() {
            let Some(chat) = self.chats.get(index) else {
                break;
            };
            if chat.project != self.chat_list.project {
                continue;
            }
            if pinned != Some(chat.pinned) && chat.pinned {
                ui.label(RichText::new("Pinned").small().weak());
            } else if pinned == Some(true) && !chat.pinned {
                ui.add_space(6.0);
                ui.label(RichText::new("Recent").small().weak());
            }
            pinned = Some(chat.pinned);
            let id = chat.id;
            let title = chat.title.clone();
            let busy = (self.chat.busy && self.chat.id == Some(id))
                || self
                    .chat_tabs
                    .parked
                    .iter()
                    .any(|tab| tab.chat.id == Some(id) && tab.chat.busy);
            let (response, more) = chat_row(
                ui,
                chat,
                Selection::from(self.chat.id == Some(id)),
                ActivityState::from(busy),
            );
            if response.clicked() {
                self.load_chat(id);
            }
            super::popup::menu(&more, 220.0).show(|ui| self.chat_menu(ui, id, &title));
            response.context_menu(|ui| {
                ui.set_width(220.0);
                self.chat_menu(ui, id, &title);
            });
        }
        if self.chats.is_empty() {
            self.empty_chat_list(ui);
        }
    }

    fn empty_chat_list(&self, ui: &mut egui::Ui) {
        let message = if !self.chat_list.filter.is_empty() {
            "No matching chats"
        } else if self.chat_list.archived {
            "No archived chats"
        } else {
            "No chats yet"
        };
        egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 8,
            })
            .show(ui, |ui| {
                ui.label(
                    RichText::new(message)
                        .size(12.0)
                        .color(Palette::for_context(ui.ctx()).muted),
                );
            });
    }

    fn chat_list_paging(&mut self, ui: &mut egui::Ui) {
        if self.chat_list.offset == 0 && !self.chat_list.has_more {
            return;
        }
        let mut offset = self.chat_list.offset;
        ui.horizontal(|ui| {
            if offset > 0 && ui.small_button("Newer chats").clicked() {
                offset = offset.saturating_sub(50);
            }
            if self.chat_list.has_more && ui.small_button("Older chats").clicked() {
                offset += 50;
            }
        });
        if offset != self.chat_list.offset {
            self.chat_list.offset = offset;
            if let Err(error) = self.refresh_chats() {
                self.error = Some(error.to_string());
            }
        }
    }
}
