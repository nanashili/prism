use crate::model::Outcome;
use crate::ui::app::Prism;

impl Prism {
    pub(super) fn can_send_reply_choice(&self) -> bool {
        self.chat.history.page.is_none()
            && !self.chat.busy
            && self.chat.queue.is_empty()
            && self.chat.permission.is_none()
            && self.chat.messages.last().is_some_and(|message| {
                message.role == "assistant"
                    && message
                        .activity
                        .as_ref()
                        .is_none_or(|activity| activity.outcome == Outcome::Done)
            })
    }

    pub(super) fn send_reply_choice(&mut self, reply: String) {
        if !self.can_send_reply_choice() {
            return;
        }

        let draft = std::mem::replace(&mut self.chat.draft, reply);
        let attachments = std::mem::take(&mut self.chat.attachments);
        let mentions = std::mem::take(&mut self.chat.mentions);

        let result = self.prepare_prompt();

        self.chat.draft = draft;
        self.chat.attachments = attachments;
        self.chat.mentions = mentions;

        if let Err(error) = result {
            self.report_error(error);
        }
        if let Err(error) = self.save_chat_draft() {
            self.report_error(error);
        }
    }
}
