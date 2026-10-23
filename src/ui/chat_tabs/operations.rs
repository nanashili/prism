use super::{
    super::{Chat, Prism},
    AgentChoice, OpenTab,
};
use crate::{error::Result, model::Outcome};

impl Prism {
    pub(in crate::ui) fn add_chat_tab(&mut self, chat: Chat) -> Result<()> {
        self.require_chat_slot()?;
        self.save_chat_draft()?;
        self.remember_selected_chat(chat.id)?;

        if !self.settings.conversation.keep_agents_running
            && let Some(agent) = &self.agent
        {
            self.chat.questions.clear_agent_question();
            agent.cancel();
        }
        self.chat_tabs.parked.push(OpenTab {
            key: self.chat_tabs.active,
            chat: std::mem::replace(&mut self.chat, chat),
            agent: self.agent.take(),
            choice: AgentChoice::capture(&self.settings),
        });
        self.chat_tabs.active = self.chat_tabs.next;
        self.chat_tabs.next += 1;
        self.chat_tabs.order.push(self.chat_tabs.active);
        self.chat_tabs.inspector = false;
        self.workspace.agent_closed = false;
        Ok(())
    }

    pub(in crate::ui) fn require_chat_slot(&self) -> Result<()> {
        if self.chat_tabs.order.len() >= 6 {
            return Err("You have 6 chats open. Close one to start another.".into());
        }
        if self.chat_tabs.retired.len() >= 6 {
            return Err("An agent is still stopping. Try again in a moment.".into());
        }
        Ok(())
    }

    pub(in crate::ui) fn select_chat_tab(&mut self, key: u64) {
        if let Err(error) = self.activate_chat_tab(key) {
            self.error = Some(error.to_string());
        }
    }

    fn activate_chat_tab(&mut self, key: u64) -> Result<()> {
        self.chat_tabs.inspector = false;
        self.workspace.agent_closed = false;
        let Some(index) = self.chat_tabs.parked.iter().position(|tab| tab.key == key) else {
            return Ok(());
        };
        self.save_chat_draft()?;
        self.remember_selected_chat(self.chat_tabs.parked[index].chat.id)?;
        if !self.settings.conversation.keep_agents_running
            && let Some(agent) = &self.agent
        {
            self.chat.questions.clear_agent_question();
            agent.cancel();
        }

        let mut tab = self.chat_tabs.parked.remove(index);
        self.exchange_chat(&mut tab);
        std::mem::swap(&mut self.chat_tabs.active, &mut tab.key);
        self.chat_tabs.parked.insert(index, tab);
        Ok(())
    }

    pub(in crate::ui) fn close_chat_tab(&mut self, key: u64) {
        if let Err(error) = self.close_open_chat(key) {
            self.error = Some(error.to_string());
        }
    }

    pub(in crate::ui) fn close_open_chat(&mut self, key: u64) -> Result<()> {
        if key == self.chat_tabs.active {
            let at = self
                .chat_tabs
                .order
                .iter()
                .position(|open| *open == key)
                .unwrap_or(0);
            if self.chat_tabs.order.len() > 1 {
                let neighbor = if at + 1 < self.chat_tabs.order.len() {
                    at + 1
                } else {
                    at - 1
                };
                self.activate_chat_tab(self.chat_tabs.order[neighbor])?;
            } else {
                self.remember_selected_chat(None)?;
                save_closing_chat(&self.store, &mut self.chat)?;
                self.retire_active_agent();
                self.chat = Chat::default();
                self.chat_tabs.active = self.chat_tabs.next;
                self.chat_tabs.next += 1;
                self.chat_tabs.order[0] = self.chat_tabs.active;
                return Ok(());
            }
        }
        if let Some(index) = self.chat_tabs.parked.iter().position(|tab| tab.key == key) {
            save_closing_chat(&self.store, &mut self.chat_tabs.parked[index].chat)?;
            let mut tab = self.chat_tabs.parked.remove(index);
            if let Some(mut agent) = tab.agent.take() {
                agent.stop();
                self.chat_tabs.retired.push(agent);
            }
            self.chat_tabs.order.retain(|open| *open != key);
        }
        Ok(())
    }

    pub(in crate::ui) fn retire_active_agent(&mut self) {
        if let Some(mut agent) = self.agent.take() {
            agent.stop();
            self.chat_tabs.retired.push(agent);
        }
    }

    pub(in crate::ui) fn save_all_chat_drafts(&self) -> Result<()> {
        let mut failures = Vec::new();
        for chat in
            std::iter::once(&self.chat).chain(self.chat_tabs.parked.iter().map(|tab| &tab.chat))
        {
            if let Some(id) = chat.id
                && let Err(error) = self
                    .store
                    .save_chat_draft(id, &chat.draft, &chat.attachments)
            {
                failures.push(error.to_string());
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("\n").into())
        }
    }

    pub(in crate::ui) fn save_chats_on_exit(&mut self) -> Result<()> {
        let mut failures = Vec::new();
        for chat in std::iter::once(&mut self.chat)
            .chain(self.chat_tabs.parked.iter_mut().map(|tab| &mut tab.chat))
        {
            if let Err(error) = save_closing_chat(&self.store, chat) {
                failures.push(error.to_string());
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("\n").into())
        }
    }

    fn remember_selected_chat(&self, id: Option<i64>) -> Result<()> {
        self.store.remember_chat(
            self.workspace
                .project
                .as_ref()
                .map(|project| project.target.sha256.as_str()),
            id,
        )
    }
}

fn save_closing_chat(store: &crate::storage::Store, chat: &mut Chat) -> Result<()> {
    let Some(id) = chat.id else {
        return Ok(());
    };
    store.save_chat_draft(id, &chat.draft, &chat.attachments)?;
    if chat.busy
        && let Some(message) = chat.messages.last()
        && message.role == "assistant"
    {
        let mut message = message.clone();
        if let Some(activity) = &mut message.activity {
            activity.finish(Outcome::Stopped);
        }
        store.append_message(id, &message)?;
        chat.busy = false;
    }
    Ok(())
}
