use super::{
    app::Prism, notifications::Level, settings_navigation::SettingsPage,
    settings_pages::model::Command,
};
use crate::storage::ChatArchive;
use crate::{error::Result, model::Screen, settings::Settings};
use eframe::egui;

impl Prism {
    pub(super) fn settings_command(&mut self, context: &egui::Context, command: Command) {
        match self.apply_settings_command(context, command) {
            Ok(message) => {
                if let Some(message) = message {
                    self.notify(Level::Success, message, "", "");
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn apply_settings_command(
        &mut self,
        context: &egui::Context,
        command: Command,
    ) -> Result<Option<&'static str>> {
        match command {
            Command::Engine(provider) => self.settings.provider = provider,
            Command::Reset => {
                let views = self.settings.views.clone();
                self.settings = Settings::default();
                self.settings_ui.custom_tool_server = None;
                self.settings.views.keymap = views.keymap;
                self.settings.views.shortcuts = views.shortcuts;
                self.debug.args.clear();
                return Ok(Some("Settings reset"));
            }
            Command::ClearChats => {
                self.require_idle_settings()?;
                self.reset_chat_tabs(Default::default())?;
                self.store.clear_chats()?;
                self.settings_ui.storage.invalidate();
                self.refresh_chats()?;
                self.recent = self.store.recent(self.settings.recent_limit)?;
                return Ok(Some("Chats deleted"));
            }
            Command::ClearCache => {
                self.require_idle_settings()?;
                self.store.trim_cache(0)?;
                self.settings_ui.storage.invalidate();
                return Ok(Some("Analysis cache cleared"));
            }
            Command::ClearRecordings => {
                self.clear_recordings()?;
                self.settings_ui.storage.invalidate();
                return Ok(Some("GPU recordings cleared"));
            }
            Command::ForgetAnswers => {
                crate::agent::remembered::forget()?;
                return Ok(Some("Remembered approvals cleared"));
            }
            Command::ReviewChats(archived) => {
                self.settings_ui.select(SettingsPage::Storage);
                self.settings_ui
                    .storage
                    .review_chats(ChatArchive::from(archived));
            }
            Command::StorageOverview => {
                self.settings_ui.select(SettingsPage::Storage);
                self.settings_ui.storage.overview();
            }
            Command::DeleteStoredChat(id) => {
                self.require_idle_settings()?;
                self.delete_chats(&[id]);
                self.settings_ui.storage.invalidate();
            }
            Command::RestoreStoredChat(id) => {
                self.store.archive_chat(id, ChatArchive::Current)?;
                self.settings_ui.storage.invalidate();
                self.refresh_chats()?;
                return Ok(Some("Chat restored"));
            }
            Command::ResetLayout => {
                self.workspace.split = false;
                self.workspace.split_fraction = None;
                self.workspace.focused_pane = 0;
                self.workspace.agent_closed = false;
                self.settings.desktop.chat_panel_width = 480.0;
                return Ok(Some("Workbench layout reset"));
            }
            Command::Setup => self.screen = Screen::Setup,
            Command::CheckUpdates => context.open_url(egui::OpenUrl::new_tab(
                "https://github.com/nanashili/prism/releases",
            )),
            Command::Open(url) => context.open_url(egui::OpenUrl::new_tab(url)),
            Command::Reveal(path) => super::settings_files::reveal(&path)?,
            Command::Choose(key) => self.choose_settings_path(key)?,
            Command::ResetPath(key) => key.value(&mut self.settings).clear(),
            Command::Agent(id, name) => self.pick_agent(context, id, name),
            Command::BrowseAgents => {
                self.settings_ui.browsing_agents = !self.settings_ui.browsing_agents;
            }
            Command::AgentVisibility(id) => {
                let hidden = &mut self.settings.conversation.hidden_agents;
                if hidden.contains(&id) {
                    hidden.retain(|agent| *agent != id);
                } else {
                    hidden.push(id);
                }
            }
            Command::Rescan => {
                self.agent_catalog.rescan();
                return Ok(Some("Agent search finished"));
            }
            Command::CustomAgent => self.add_custom_agent()?,
            Command::CustomToolServer => return self.save_custom_tool_server().map(Some),
            Command::NewSkill => self.create_skill()?,
            Command::Skill(id, enabled) => {
                self.settings
                    .desktop
                    .disabled_skills
                    .retain(|disabled| *disabled != id);
                if !enabled {
                    self.settings.desktop.disabled_skills.push(id);
                }
            }
            Command::Permission(tool, level) => self.settings.permissions.set(tool, level),
            Command::CopyDetails => {
                context.copy_text(format!("Prism {}\n{} {}\nEngine: {}\nREA: {}\nRizin: {}\nDebugger: {}\nRecent error: {}",
                    env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH,
                    self.settings.provider, self.settings.rea, self.settings.rizin, self.settings.lldb,
                    self.error.as_deref().or_else(|| self.notifications.recent_error()).unwrap_or("None")));
                return Ok(Some("Details copied"));
            }
        }
        Ok(None)
    }

    fn save_custom_tool_server(&mut self) -> Result<&'static str> {
        let command = self
            .settings_ui
            .custom_tool_server
            .as_deref()
            .unwrap_or(&self.settings.conversation.custom_tool_server)
            .trim()
            .to_owned();
        crate::agent::tool_servers::custom_configuration(&command)?;

        let removed = command.is_empty();
        self.settings.conversation.custom_tool_server = command.clone();
        self.settings_ui.custom_tool_server = Some(command);
        Ok(if removed {
            "Removed the tool server"
        } else {
            "Added the tool server. It starts with your next message."
        })
    }

    fn require_idle_settings(&self) -> Result<()> {
        if self.chat_tabs.any_busy(&self.chat) || self.pending > 0 {
            return Err("Wait for the current operation before clearing stored data".into());
        }
        Ok(())
    }
}
