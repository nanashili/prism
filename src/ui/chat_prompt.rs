use super::app::Prism;
use crate::agent::Prompt;
use crate::bridge::{AnalysisTools, WindowTools};
use crate::model::{Attachment, Message};

impl Prism {
    pub(super) fn run_playbook(&mut self, playbook: &super::playbooks::Playbook) {
        self.chat.draft = playbook.with_reference(&self.chat.draft);
        self.send_prompt();
    }

    pub(super) fn prepare_prompt(&mut self) -> crate::error::Result<()> {
        self.remember_selected_model()?;
        let title = if self.chat.draft.trim().is_empty() {
            self.chat
                .attachments
                .first()
                .map_or("New chat", |item| &item.name)
        } else {
            self.chat.draft.trim()
        }
        .to_owned();
        let title = super::playbooks::shown_text(&title);
        let chat = self.chat_for_prompt(&title)?;
        self.store.save_chat_models(chat, &self.chat.models)?;
        let draft = super::composer::mentions::with_mentions(&self.chat.draft, &self.chat.mentions);
        let text = Attachment::message_text(&draft, &self.chat.attachments);
        let prompt = self.agent_prompt(&text)?;
        let mut message = Message {
            id: None,
            role: "user".into(),
            text,
            activity: None,
        };

        if self.agent.is_none() {
            self.agent = Some(crate::agent::Agent::start(
                self.chat_tabs.sink.for_chat(self.chat_tabs.active),
            ));
        }
        let agent = self.agent.as_ref().ok_or("The agent worker is closed")?;
        message.id = Some(self.store.append_message(chat, &message)?);
        if let Err(error) = agent.prompt(prompt) {
            return match self.save_chat_draft() {
                Ok(()) => Err(error),
                Err(recovery) => Err(crate::error::failure(format!(
                    "{error}. Couldn't save the draft: {recovery}"
                ))),
            };
        }

        self.begin_agent_turn(chat, message)
    }

    pub(super) fn prepare_agent_session(&mut self) {
        if self.chat.connected
            || self.chat.busy
            || self
                .agent
                .as_ref()
                .is_some_and(crate::agent::Agent::is_preparing)
        {
            return;
        }
        let result = self.connect_agent_session();
        if let Err(error) = result {
            self.report_error(error);
        }
    }

    fn connect_agent_session(&mut self) -> crate::error::Result<()> {
        let mut prompt = self.agent_prompt("")?;
        prompt.attachments.clear();
        // Session preparation discovers capabilities; it never submits a prompt.
        if self.agent.is_none() {
            self.agent = Some(crate::agent::Agent::start(
                self.chat_tabs.sink.for_chat(self.chat_tabs.active),
            ));
        }
        self.agent
            .as_ref()
            .ok_or("The agent worker is closed")?
            .prepare(prompt)
    }

    pub(super) fn chat_for_prompt(&mut self, text: &str) -> crate::error::Result<i64> {
        if let Some(id) = self.chat.id {
            return Ok(id);
        }
        let title: String = text.chars().take(60).collect();
        let id = self.store.new_chat(
            self.workspace
                .project
                .as_ref()
                .map(|project| project.target.sha256.as_str()),
            &title,
        )?;
        self.chat.id = Some(id);
        self.chat.title = title;
        Ok(id)
    }

    fn agent_prompt(&mut self, text: &str) -> crate::error::Result<Prompt> {
        // Resolve the transport copy; drafts and saved messages keep references.
        let text = self.playbooks.resolve_prompt(&self.settings, text)?;
        let cwd = self
            .workspace
            .project
            .as_ref()
            .and_then(|project| project.target.path.parent())
            .map_or(crate::settings::support_dir(), std::path::Path::to_path_buf);
        let servers = self
            .bridge
            .as_ref()
            .ok_or("The window bridge is closed")?
            .configurations(
                WindowTools::from(self.settings.conversation.prism_tools),
                AnalysisTools::from(self.settings.conversation.rea_tools),
                self.chat_tabs.active,
            )?;
        let servers = crate::agent::tool_servers::with_custom(
            servers,
            &self.settings.conversation.custom_tool_server,
        )?;

        Ok(Prompt {
            chat: self.chat_tabs.active,
            project: self
                .workspace
                .project
                .as_ref()
                .map(|project| project.target.sha256.clone()),
            file: self
                .workspace
                .project
                .as_ref()
                .map(|project| project.target.path.clone()),
            text,
            attachments: self.chat.attachments.clone(),
            context: self.prompt_context(),
            history: self.prompt_history(),
            cwd,
            settings: self.settings.clone(),
            servers,
            picks: self.prompt_option_picks(),
        })
    }

    fn begin_agent_turn(&mut self, chat: i64, message: Message) -> crate::error::Result<()> {
        self.chat.history.page = None;
        self.chat.messages.push(message);
        self.chat.messages.push(Message {
            id: None,
            role: "assistant".into(),
            text: String::new(),
            activity: Some(crate::model::Activity::start(super::agent_picker::id(
                &self.settings,
            ))),
        });
        self.chat.draft.clear();
        self.chat.attachments.clear();
        self.chat.mentions.clear();
        self.chat.busy = true;
        self.chat.status = "Connecting to agent".into();
        self.store.remember_chat(
            self.workspace
                .project
                .as_ref()
                .map(|project| project.target.sha256.as_str()),
            Some(chat),
        )?;
        self.refresh_chats()
    }
}
