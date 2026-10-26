use super::super::{settings_navigation::SettingsPage, settings_pages, theme::Palette};
use crate::storage::ChatArchive;
use crate::storage::{Store, StoredChat};
use eframe::egui::{self, RichText};
use settings_pages::model::{Command, Group, Row};

pub(super) struct Review {
    pub archived: bool,
    filter: String,
    offset: usize,
    rows: Vec<StoredChat>,
    has_more: bool,
    loaded: bool,
    error: Option<String>,
}

impl Review {
    pub fn new(archived: ChatArchive) -> Self {
        let archived = archived == ChatArchive::Archived;

        Self {
            archived,
            filter: String::new(),
            offset: 0,
            rows: Vec::new(),
            has_more: false,
            loaded: false,
            error: None,
        }
    }

    pub fn invalidate(&mut self) {
        self.loaded = false;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        store: &Store,
        confirming: &mut Option<Command>,
    ) -> Option<Command> {
        let action = self.header(ui, confirming);
        self.search(ui, confirming);
        if !self.loaded {
            self.load(store);
        }
        ui.add_space(16.0);
        let command = self.chat_rows(ui, confirming);
        ui.add_space(12.0);
        self.pager(ui, confirming);
        command.or(action)
    }

    fn header(&mut self, ui: &mut egui::Ui, confirming: &mut Option<Command>) -> Option<Command> {
        let mut action = None;
        ui.horizontal(|ui| {
            if super::button(ui, "Back to storage").clicked() {
                action = Some(Command::StorageOverview);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if super::button(ui, "Refresh").clicked() {
                    self.invalidate();
                    *confirming = None;
                }
            });
        });
        ui.add_space(16.0);
        action
    }

    fn search(&mut self, ui: &mut egui::Ui, confirming: &mut Option<Command>) {
        if ui
            .add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("Search chats across projects")
                    .char_limit(200)
                    .desired_width(ui.available_width()),
            )
            .changed()
        {
            self.offset = 0;
            self.invalidate();
            *confirming = None;
        }
    }

    fn chat_rows(&self, ui: &mut egui::Ui, confirming: &mut Option<Command>) -> Option<Command> {
        let p = Palette::for_context(ui.ctx());
        if let Some(error) = &self.error {
            ui.label(RichText::new(format!("Couldn’t load chats: {error}")).color(p.muted));
        } else if self.rows.is_empty() {
            ui.label(RichText::new("No chats to show.").color(p.muted));
        } else {
            let rows = self.rows.iter().map(review_row).collect();
            return settings_pages::show(
                ui,
                vec![Group::new("", rows)],
                SettingsPage::Storage,
                "",
                confirming,
            )
            .0;
        }
        None
    }

    fn load(&mut self, store: &Store) {
        self.rows.clear();
        self.has_more = false;
        self.error = None;
        match store.stored_chats(&self.filter, ChatArchive::from(self.archived), self.offset) {
            Ok(mut rows) => {
                self.has_more = rows.len() > 50;
                rows.truncate(50);
                self.rows = rows;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        self.loaded = true;
    }

    fn pager(&mut self, ui: &mut egui::Ui, confirming: &mut Option<Command>) {
        ui.horizontal(|ui| {
            let mut offset = self.offset;
            if ui
                .add_enabled_ui(self.offset > 0, |ui| super::button(ui, "Previous"))
                .inner
                .clicked()
            {
                offset = self.offset.saturating_sub(50);
            }
            ui.label(format!("Page {}", self.offset / 50 + 1));
            if ui
                .add_enabled_ui(self.has_more, |ui| super::button(ui, "Next"))
                .inner
                .clicked()
            {
                offset = self.offset.saturating_add(50);
            }
            if offset != self.offset {
                self.offset = offset;
                self.invalidate();
                *confirming = None;
            }
        });
    }
}

fn review_row(chat: &StoredChat) -> Row<'static> {
    let description = format!(
        "{} · Last active {}",
        chat.project_title,
        crate::ui::sidebar_rows::relative_time(chat.updated)
    );
    let mut row = Row::new(&chat.title, description);
    if chat.archived {
        row = row.button("Restore", Command::RestoreStoredChat(chat.id));
    }
    row.button("Delete…", Command::DeleteStoredChat(chat.id))
}
