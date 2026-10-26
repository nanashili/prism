use crate::model::{Project, Screen, View};
use crate::ui::{app::Prism, notifications::Level, presentation::ChatOrigin};
use eframe::egui;
use std::path::{Path, PathBuf};

impl Prism {
    pub(super) fn receive_opened(
        &mut self,
        context: &egui::Context,
        project: Project,
        origin: ChatOrigin,
    ) {
        if self.screen == Screen::Setup {
            self.settings.research.setup_complete = true;
            self.queue_settings_save(context);
        }
        self.reconstruction = Default::default();
        self.reconstruction_root = None;
        self.debug.isolated = false;
        self.restore_document_chat(&project, origin);

        self.workspace = crate::ui::app::Workspace {
            project: Some(project),
            ..Default::default()
        };
        self.restore_sidebar_preferences();
        self.investigation.objective.clear();
        if let Err(error) = self.refresh_investigation(None) {
            self.report_error(error);
        }
        if self.settings.conversation.notify_when_ready {
            self.notify(
                Level::Success,
                "File ready",
                "Your file is ready to explore.",
                "",
            );
        }
        self.screen = if self.settings.conversation.open_workbench == "Only when I ask"
            || (self.chat.busy && self.settings.conversation.open_workbench != "Always")
        {
            Screen::Home
        } else {
            Screen::Workbench
        };
        if let Err(error) = self.refresh_chats() {
            self.report_error(error);
        }
        match self.store.recent(self.settings.recent_limit) {
            Ok(recent) => self.recent = recent,
            Err(error) => self.report_error(error),
        }
    }

    pub(super) fn receive_content(
        &mut self,
        request: Option<u64>,
        view: View,
        address: &str,
        text: String,
    ) {
        let active = self.workspace.active;
        let pane = if let Some(request) = request {
            self.workspace
                .panes
                .iter_mut()
                .flat_map(|pane| pane.tabs.iter_mut())
                .find(|tab| request == tab.request && !tab.tab_closed)
        } else {
            let Some(tab) = self.workspace.tabs.get_mut(active) else {
                return;
            };
            tab.view = view;
            tab.tab_closed = false;
            Some(tab)
        };
        if let Some(pane) = pane
            && let Err(error) = pane.receive_content(view, address, text)
        {
            self.report_error(error);
        }
    }

    pub(super) fn receive_exported(&mut self, path: &Path) {
        self.notify(
            Level::Success,
            "Export saved",
            &path.display().to_string(),
            "",
        );
        if self.settings.desktop.reveal_exports
            && let Err(error) = crate::ui::settings_files::reveal(path)
        {
            self.notify(
                Level::Warning,
                "Couldn't show the export",
                "Your export was saved. Finder could not open its location.",
                &error.to_string(),
            );
        }
    }

    pub(super) fn receive_saved(&mut self, path: PathBuf) {
        if let Some(project) = &mut self.workspace.project {
            project.source_document = Some(path.clone());
        }
        self.notify(
            Level::Success,
            "Project saved",
            &path.display().to_string(),
            "",
        );
        self.workspace.document = Some(path);
    }
}
