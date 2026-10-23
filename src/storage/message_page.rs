use super::Store;
use crate::error::{Result, failure};
use crate::model::Message;
use rusqlite::params;

const PAGE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) enum MessageWindow {
    Latest,
    Before(i64),
    After(i64),
}

pub(crate) struct MessagePage {
    pub messages: Vec<Message>,
    pub has_earlier: bool,
    pub has_later: bool,
}

impl Store {
    pub fn message_page(&self, chat: i64, window: MessageWindow) -> Result<MessagePage> {
        let (comparison, cursor, order) = match window {
            MessageWindow::Latest => ("<=", i64::MAX, "DESC"),
            MessageWindow::Before(id) => ("<", id, "DESC"),
            MessageWindow::After(id) => (">", id, "ASC"),
        };
        let query = format!(
            "SELECT m.id,m.role,
             coalesce(substr(cast(m.text AS BLOB),1,1048576),x''),
             substr(cast(a.data AS BLOB),1,2097153)
             FROM messages m LEFT JOIN message_activity a ON a.message=m.id
             WHERE m.chat=?1 AND m.id {comparison} ?2 ORDER BY m.id {order} LIMIT 40"
        );
        let mut statement = self.db.prepare(&query)?;
        let mut rows = statement.query(params![chat, cursor])?;
        let mut messages = read_page(&mut rows)?;
        if !matches!(window, MessageWindow::After(_)) {
            messages.reverse();
        }

        let (has_earlier, has_later) = self.message_boundaries(chat, &messages)?;
        Ok(MessagePage {
            messages,
            has_earlier,
            has_later,
        })
    }

    fn message_boundaries(&self, chat: i64, messages: &[Message]) -> Result<(bool, bool)> {
        let Some(first) = messages.first().and_then(|message| message.id) else {
            return Ok((false, false));
        };
        let Some(last) = messages.last().and_then(|message| message.id) else {
            return Ok((false, false));
        };
        Ok(self.db.query_row(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE chat=?1 AND id<?2),
                    EXISTS(SELECT 1 FROM messages WHERE chat=?1 AND id>?3)",
            params![chat, first, last],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    }
}

fn read_page(rows: &mut rusqlite::Rows<'_>) -> Result<Vec<Message>> {
    let mut messages = Vec::new();
    let mut bytes = 0;
    while let Some(row) = rows.next()? {
        let text: Vec<u8> = row.get(2)?;
        let activity: Option<Vec<u8>> = row.get(3)?;
        let size = text.len() + activity.as_ref().map_or(0, Vec::len);
        if bytes + size > PAGE_BYTES {
            if messages.is_empty() {
                return Err(failure(
                    "This saved message exceeds the 2 MiB history page limit",
                ));
            }
            break;
        }

        bytes += size;
        messages.push(Message {
            id: Some(row.get(0)?),
            role: row.get(1)?,
            text: String::from_utf8(text)
                .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned()),
            activity: activity
                .map(|data| serde_json::from_slice(&data))
                .transpose()?,
        });
    }
    Ok(messages)
}
