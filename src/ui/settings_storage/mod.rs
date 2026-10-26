use crate::storage::ChatArchive;
mod chats;
mod measurement;
mod overview;

use super::{app::Prism, settings_pages::model::Command};
use eframe::egui;

#[derive(Default)]
pub(super) struct StorageUi {
    measurement: measurement::Measurement,
    review: Option<chats::Review>,
}

impl StorageUi {
    pub fn title(&self) -> &'static str {
        match &self.review {
            Some(review) if review.archived => "Archived chats",
            Some(_) => "Review chats",
            None => "This device",
        }
    }

    pub fn intro(&self) -> &'static str {
        if self.review.is_some() {
            "Chats from every project on this computer, with the oldest first."
        } else {
            "Manage projects, chats and analysis stored on this computer."
        }
    }

    pub fn review_chats(&mut self, archived: ChatArchive) {
        self.review = Some(chats::Review::new(archived));
    }

    pub fn overview(&mut self) {
        self.review = None;
    }

    pub fn invalidate(&mut self) {
        self.measurement.invalidate();
        if let Some(review) = &mut self.review {
            review.invalidate();
        }
    }
}

impl Prism {
    pub(super) fn storage_settings(&mut self, ui: &mut egui::Ui) -> bool {
        if let Some(review) = &mut self.settings_ui.storage.review {
            if let Some(command) = review.show(ui, &self.store, &mut self.settings_ui.confirm) {
                self.settings_command(ui.ctx(), command);
            }
            return true;
        }

        overview::show(ui, &mut self.settings_ui.storage.measurement);
        false
    }
}

fn button(ui: &mut egui::Ui, title: &str) -> egui::Response {
    super::settings_pages::action_button(
        ui,
        &super::settings_pages::model::Button::new(title, Command::StorageOverview),
    )
}
