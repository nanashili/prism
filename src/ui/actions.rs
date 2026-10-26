use super::app::{Chat, Prism};
use crate::ui::presentation::{OpenTarget, SaveDestination};
use crate::worker::Job;

impl Prism {
    pub(super) fn choose_open(&mut self, folder: OpenTarget) {
        let folder = folder == OpenTarget::Folder;

        let dialog =
            rfd::FileDialog::new().set_title("Open an app, binary, shader, model or project");
        let path = if folder {
            dialog.pick_folder()
        } else {
            dialog.pick_file()
        };
        if let Some(path) = path {
            self.enqueue(Job::Open(path));
        }
    }

    pub(super) fn new_chat(&mut self) {
        if let Err(error) = self.add_chat_tab(Chat::default()) {
            self.error = Some(error.to_string());
        }
    }

    pub(super) fn load_chat(&mut self, id: i64) {
        if self.chat.id == Some(id) {
            self.select_chat_tab(self.chat_tabs.active);
            return;
        }
        if let Some(tab) = self
            .chat_tabs
            .parked
            .iter()
            .find(|tab| tab.chat.id == Some(id))
        {
            self.select_chat_tab(tab.key);
            return;
        }
        let result = (|| -> crate::error::Result<_> {
            self.require_chat_slot()?;
            self.save_chat_draft()?;
            let project_key = self.store.chat_project(id)?;
            let current = self
                .workspace
                .project
                .as_ref()
                .map(|project| project.target.sha256.as_str());
            let chat = Chat::restore(&self.store, id)?;
            let moving = project_key.as_deref() != current;
            if moving {
                if self.chat_tabs.any_busy(&self.chat) || self.pending > 0 {
                    return Err(crate::error::failure(
                        "Wait for the current operation before changing projects",
                    ));
                }
                self.save_inspector_note()?;
                self.save_all_chat_drafts()?;
                if let Some(key) = &project_key {
                    let project = self.store.project(key)?;
                    self.submit(Job::Open(
                        project.source_document.unwrap_or(project.target.path),
                    ))?;
                } else {
                    self.workspace = Default::default();
                }
            }
            Ok((chat, moving, project_key))
        })();
        match result {
            Ok((chat, moving, project)) => {
                let result = if moving {
                    self.reset_chat_tabs(chat)
                } else {
                    self.add_chat_tab(chat)
                };
                if let Err(error) =
                    result.and_then(|()| self.store.remember_chat(project.as_deref(), Some(id)))
                {
                    self.error = Some(error.to_string());
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(super) fn save_document(&mut self, choose: SaveDestination) {
        let choose = choose == SaveDestination::Choose;

        let Some(project) = self.workspace.project.clone() else {
            return;
        };
        let path = if choose || self.workspace.document.is_none() {
            rfd::FileDialog::new()
                .set_title("Save project")
                .add_filter("Prism project", &["prism"])
                .set_file_name(format!("{}.prism", project.target.name))
                .save_file()
        } else {
            self.workspace.document.clone()
        };
        if let Some(path) = path {
            self.enqueue(Job::Save {
                path,
                project,
                messages: self.chat.messages.clone(),
            });
        }
    }

    pub(super) fn send_prompt(&mut self) {
        if self.send_goal_control() {
            return;
        }
        if self.chat.draft.trim().is_empty() && self.chat.attachments.is_empty() {
            return;
        }
        let result = if self.chat.busy || !self.chat.queue.is_empty() {
            self.enqueue_follow_up()
        } else {
            self.prepare_prompt()
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
        } else if self.screen == crate::model::Screen::Setup {
            self.screen = crate::model::Screen::Home;
        }
    }
}
