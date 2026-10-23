use super::super::{Chat, Prism};
use crate::error::Result;

impl Prism {
    pub(in crate::ui) fn disconnect_chat_agent(&mut self) -> Result<()> {
        if self.chat.busy {
            return Err("Stop the current turn before changing agents".into());
        }
        if self.agent.is_some() && self.chat_tabs.retired.len() >= 6 {
            return Err("An agent is still stopping. Try again in a moment.".into());
        }

        self.retire_active_agent();
        let previous = self.chat_tabs.active;
        self.chat_tabs.active = self.chat_tabs.next;
        self.chat_tabs.next += 1;
        for key in &mut self.chat_tabs.order {
            if *key == previous {
                *key = self.chat_tabs.active;
            }
        }

        self.chat.connected = false;
        self.chat.permission = None;
        self.chat.questions.clear_agent_question();
        self.chat.goal = None;
        self.chat.usage = None;
        self.chat.commands.clear();
        self.chat.options = serde_json::Value::Null;
        self.chat.picks.clear();
        self.chat.composer_revision += 1;
        Ok(())
    }

    pub(in crate::ui) fn reset_chat_tabs(&mut self, chat: Chat) -> Result<()> {
        self.save_all_chat_drafts()?;
        self.retire_active_agent();
        for mut tab in self.chat_tabs.parked.drain(..) {
            if let Some(mut agent) = tab.agent.take() {
                agent.stop();
                self.chat_tabs.retired.push(agent);
            }
        }
        self.chat = chat;
        self.chat_tabs.active = self.chat_tabs.next;
        self.chat_tabs.next += 1;
        self.chat_tabs.order = vec![self.chat_tabs.active];
        self.chat_tabs.inspector = false;
        Ok(())
    }
}
