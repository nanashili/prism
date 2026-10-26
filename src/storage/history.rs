use super::Store;
use crate::error::Result;
use crate::model::{Attachment, Message, QueuedPrompt};
use rusqlite::{OptionalExtension, params};
use std::collections::VecDeque;

impl Store {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn save_chat_draft(
        &self,
        chat: i64,
        draft: &str,
        attachments: &[Attachment],
    ) -> Result<()> {
        if draft.is_empty() && attachments.is_empty() {
            self.db
                .execute("DELETE FROM chat_drafts WHERE chat=?1", [chat])?;
        } else {
            self.db.execute(
                "INSERT INTO chat_drafts(chat,draft,attachments) VALUES(?1,?2,?3)
                 ON CONFLICT(chat) DO UPDATE SET draft=?2,attachments=?3",
                params![chat, draft, serde_json::to_string(attachments)?],
            )?;
        }
        Ok(())
    }

    pub fn chat_draft(&self, chat: i64) -> Result<(String, Vec<Attachment>)> {
        let saved = self
            .db
            .query_row(
                "SELECT draft,substr(cast(attachments AS BLOB),1,262145)
                 FROM chat_drafts WHERE chat=?1",
                [chat],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()?;
        let Some((draft, bytes)) = saved else {
            return Ok(Default::default());
        };
        if bytes.len() > 262144 {
            return Err("The saved attachment list exceeds its size limit".into());
        }
        let saved: Vec<Attachment> = serde_json::from_slice(&bytes)?;
        if saved.len() > Attachment::LIMIT {
            return Err("The saved draft has too many attachments".into());
        }
        let attachments = saved
            .iter()
            .map(|attachment| Attachment::from_location(&attachment.uri))
            .collect::<Result<_>>()?;
        Ok((draft, attachments))
    }

    pub fn save_chat_queue(&self, chat: i64, queue: &VecDeque<QueuedPrompt>) -> Result<()> {
        if queue.is_empty() {
            self.db
                .execute("DELETE FROM chat_queue WHERE chat=?1", [chat])?;
            return Ok(());
        }
        let data = serde_json::to_string(queue)?;
        if data.len() > QueuedPrompt::BYTE_LIMIT || queue.len() > QueuedPrompt::LIMIT {
            return Err("The follow-up queue is full. Send or remove an item first.".into());
        }
        self.db.execute(
            "INSERT INTO chat_queue(chat,data) VALUES(?1,?2)
             ON CONFLICT(chat) DO UPDATE SET data=?2",
            params![chat, data],
        )?;
        Ok(())
    }

    pub fn save_queue_and_draft(
        &self,
        chat: i64,
        queue: &VecDeque<QueuedPrompt>,
        draft: &str,
        attachments: &[Attachment],
    ) -> Result<()> {
        let transaction = self.db.unchecked_transaction()?;
        self.save_chat_queue(chat, queue)?;
        self.save_chat_draft(chat, draft, attachments)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn chat_queue(&self, chat: i64) -> Result<VecDeque<QueuedPrompt>> {
        let bytes: Option<Vec<u8>> = self
            .db
            .query_row(
                "SELECT substr(cast(data AS BLOB),1,?2) FROM chat_queue WHERE chat=?1",
                params![chat, i64::try_from(QueuedPrompt::BYTE_LIMIT + 1)?],
                |row| row.get(0),
            )
            .optional()?;
        match bytes {
            Some(bytes) => QueuedPrompt::decode(&bytes),
            None => Ok(VecDeque::new()),
        }
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn append_message(&self, chat: i64, message: &Message) -> Result<i64> {
        let transaction = self.db.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO messages(chat,role,text) VALUES(?1,?2,?3)",
            params![chat, message.role, message.text],
        )?;
        let id = transaction.last_insert_rowid();

        if message.role == "user" {
            transaction.execute("DELETE FROM chat_drafts WHERE chat=?1", [chat])?;
        }
        if let Some(activity) = &message.activity {
            transaction.execute(
                "INSERT INTO message_activity(message,data) VALUES(?1,?2)",
                params![id, serde_json::to_string(activity)?],
            )?;
        }
        transaction.execute("UPDATE chats SET updated=unixepoch() WHERE id=?1", [chat])?;
        transaction.commit()?;
        Ok(id)
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn update_message_activity(
        &self,
        message: i64,
        activity: &crate::model::Activity,
    ) -> Result<()> {
        let updated = self.db.execute(
            "UPDATE message_activity SET data=?2 WHERE message=?1",
            params![message, serde_json::to_string(activity)?],
        )?;
        if updated != 1 {
            return Err("The saved conversation activity could not be updated".into());
        }
        Ok(())
    }

    pub fn chat_project(&self, chat: i64) -> Result<Option<String>> {
        Ok(self
            .db
            .query_row("SELECT project FROM chats WHERE id=?1", [chat], |row| {
                row.get(0)
            })?)
    }
}
