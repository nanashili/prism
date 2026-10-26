use super::app::{Chat, Prism};
use crate::ui::presentation::{ChatOrigin, TurnEnd};

impl Prism {
    pub(super) fn append_agent_text(&mut self, text: String) {
        if let Some(message) = self.chat.messages.last_mut() {
            let remaining = (1024 * 1024usize).saturating_sub(message.text.len());
            if remaining == 0 {
                if let Some(agent) = &self.agent {
                    agent.cancel();
                }
                self.error = Some(
                    "This answer reached the 1 MiB memory limit. Start another turn to continue."
                        .into(),
                );
                return;
            }
            let mut end = text.len().min(remaining);
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            let start = message.text.len();
            message.text.push_str(&text[..end]);
            if let Some(activity) = &mut message.activity {
                activity.record_text(start, message.text.len());
            }
        }
    }
    pub(super) fn finish_agent(&mut self, result: crate::error::Result<()>, stopped: TurnEnd) {
        let stopped = stopped == TurnEnd::Stopped;

        use crate::model::Outcome;
        self.chat.busy = false;
        self.chat.permission = None;
        self.chat.questions.clear_agent_question();
        self.chat.status.clear();
        let outcome = if stopped {
            Outcome::Stopped
        } else if result.is_ok() {
            Outcome::Done
        } else {
            Outcome::Failed
        };
        let saved = self.save_agent_reply(outcome);
        let continue_queue = result.is_ok() && !stopped && saved.is_ok();
        if let Err(error) = saved {
            self.report_error(error);
        }
        if !stopped && let Err(error) = result {
            self.report_error(error);
        }
        self.chat.trim_history();
        if let Err(error) = self.refresh_chats() {
            self.report_error(error);
        }
        if continue_queue {
            self.send_next_follow_up();
        }
    }

    fn save_agent_reply(&mut self, outcome: crate::model::Outcome) -> crate::error::Result<()> {
        let Some(message) = self
            .chat
            .messages
            .last_mut()
            .filter(|message| message.role == "assistant")
        else {
            return Ok(());
        };
        if let Some(activity) = &mut message.activity {
            activity.progress.goal.clone_from(&self.chat.goal);
            activity.finish(outcome);
        }
        if let Some(chat) = self.chat.id {
            message.id = Some(self.store.append_message(chat, message)?);
        }
        Ok(())
    }

    pub(super) fn restore_document_chat(
        &mut self,
        project: &crate::model::Project,
        document: ChatOrigin,
    ) {
        let document = document == ChatOrigin::Document;

        if let Some(id) = self.chat.id {
            match self.store.chat_project(id) {
                Ok(Some(key)) if key == project.target.sha256 => return,
                Ok(_) => {}
                Err(error) => {
                    self.report_error(error);
                    return;
                }
            }
        }
        if let Err(error) = self.reset_chat_tabs(Chat::default()) {
            self.report_error(error);
            return;
        }
        match self.restore_saved_chat(Some(&project.target.sha256)) {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                self.report_error(error);
                return;
            }
        }
        if document && let Some(value) = project.imported.get("rust-chat.json") {
            match serde::Deserialize::deserialize(value.as_ref()) {
                Ok(messages) => {
                    self.chat = super::app::Chat {
                        messages,
                        ..Default::default()
                    };
                    self.chat.trim_history();
                }
                Err(error) => {
                    self.error = Some(format!("Saved chat could not be restored: {error}"));
                }
            }
        }
    }
    pub(super) fn restore_saved_chat(
        &mut self,
        project: Option<&str>,
    ) -> crate::error::Result<bool> {
        let Some(id) = self.store.selected_chat(project)? else {
            return Ok(false);
        };
        self.chat = Chat::restore(&self.store, id)?;
        Ok(true)
    }

    pub(super) fn save_chat_draft(&self) -> crate::error::Result<()> {
        if let Some(id) = self.chat.id {
            self.store
                .save_chat_draft(id, &self.chat.draft, &self.chat.attachments)?;
        }
        Ok(())
    }

    pub(super) fn prompt_context(&self) -> crate::agent::PromptContext {
        let mut static_instructions =
            String::from("You are working in Prism, a reverse engineering workbench.\n");
        static_instructions.push_str("Use the prism MCP server to control this window and prism-analysis for decompile_batch and other analysis. For unfamiliar analysis calls, first request a compact summary with analysis_tools, then request detail=full with names:[...] before invoking analysis_call with the advertised schema. Reconstruction tools are on prism. Do not assume other analysis servers are available.\n");
        static_instructions.push_str(prism::mcp::BATCH_INSTRUCTIONS);
        static_instructions.push('\n');
        static_instructions.push_str(prism::mcp::INVESTIGATION_INSTRUCTIONS);
        static_instructions.push('\n');
        static_instructions.push_str(prism::mcp::RESPONSE_INSTRUCTIONS);
        static_instructions.push('\n');

        let mut dynamic_context = String::new();
        if let Some(graph) = &self.investigation.graph {
            dynamic_context.push_str(&format!("Saved Prism investigation: {}. Read its status to recover the graph before continuing its objective.\n", graph.id));
        }
        if self.settings.conversation.prism_tools {
            dynamic_context.push_str("When you need a choice, clarification or other user input, call request_user_input on prism. Provide concise options or omit them for a free-text question. Wait for its response; skipped or dismissed means no answer and never approval.\n");
        }
        dynamic_context.push_str("Selected analysis provider: ");
        dynamic_context.push_str(&self.settings.provider);
        dynamic_context.push('\n');
        if let Some(project) = &self.workspace.project {
            dynamic_context.push_str(&format!(
                "Opened file: {}\nSHA-256: {}\nSelected function: {}\n",
                project.target.path.display(),
                project.target.sha256,
                self.workspace.selected
            ));
            if let Some(name) = project.renamed.get(&self.workspace.selected) {
                dynamic_context.push_str("Local function name: ");
                dynamic_context.push_str(name);
                dynamic_context.push('\n');
            }
            if let Some(note) = project.notes.get(&self.workspace.selected) {
                dynamic_context.push_str("Function note:\n");
                dynamic_context.extend(note.chars().take(4000));
                dynamic_context.push('\n');
            }
        }
        if !self.workspace.text.is_empty() {
            dynamic_context.push_str("Visible code or data:\n");
            dynamic_context.extend(self.workspace.text.chars().take(16_000));
            dynamic_context.push('\n');
        }
        crate::agent::PromptContext {
            static_instructions,
            dynamic_context,
        }
    }

    pub(super) fn prompt_history(&self) -> String {
        let mut text = String::new();
        let limit = match self.settings.conversation.history.as_str() {
            "Short" => 6,
            "Long" => 40,
            _ => 20,
        };
        let start = self.chat.messages.len().saturating_sub(limit);
        for message in &self.chat.messages[start..] {
            text.push_str(&format!("\n{}: ", message.role));
            text.extend(message.text.chars().take(2000));
        }
        text
    }
}
