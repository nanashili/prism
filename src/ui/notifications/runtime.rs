use super::{Level, state::Notice};
use crate::ui::app::Prism;
use crate::ui::presentation::TurnEnd;
use eframe::egui;

impl Prism {
    pub(in crate::ui) fn notify(&mut self, level: Level, title: &str, body: &str, details: &str) {
        let scope = self
            .workspace
            .project
            .as_ref()
            .map_or("", |project| project.target.name.as_str());
        self.notifications
            .push(Notice::new(level, title, body, details, scope));
    }

    pub(in crate::ui) fn report_error(&mut self, error: impl std::fmt::Display) {
        let details = error.to_string();
        let body = if details.contains("Invalid column type Null") {
            "A saved chat couldn't be loaded. Your stored messages have not been changed."
        } else {
            details
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("An unexpected error occurred. Open the details for more information.")
        };
        self.notify(Level::Error, "Couldn't complete that", body, &details);
    }

    pub(in crate::ui) fn capture_error(&mut self) {
        if let Some(error) = self.error.take() {
            self.report_error(error);
        }
    }

    pub(in crate::ui) fn notify_agent_complete(&mut self, stopped: TurnEnd) {
        let stopped = stopped == TurnEnd::Stopped;

        let title = if stopped {
            "Agent stopped"
        } else {
            "Reply ready"
        };
        let body = self.chat.title.clone();
        let level = if stopped { Level::Info } else { Level::Success };
        self.notify(level, title, &body, "");
    }

    pub(in crate::ui) fn notify_permission(&mut self, title: &str) {
        let scope = self
            .workspace
            .project
            .as_ref()
            .map_or("", |project| project.target.name.as_str());
        let mut notice = Notice::new(Level::Warning, "The agent needs your OK", title, "", scope);
        notice.chat = self.chat.id;
        self.notifications.push(notice);
    }

    pub(in crate::ui) fn show_notifications(&mut self, context: &egui::Context) {
        self.capture_error();
        if self.screen == crate::model::Screen::Settings
            || (self.screen == crate::model::Screen::Workbench && self.workspace.navigation.hidden)
        {
            egui::Area::new("notification-button".into())
                .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -8.0])
                .order(egui::Order::Foreground)
                .show(context, |ui| self.notifications.button(ui));
        }
        if let Some(id) = self.notifications.show(context) {
            self.load_chat(id);
            self.workspace.agent_closed = false;
            self.screen = if self.workspace.project.is_some() {
                crate::model::Screen::Workbench
            } else {
                crate::model::Screen::Home
            };
            self.notifications.open = false;
            context.request_repaint();
        }
    }
}
