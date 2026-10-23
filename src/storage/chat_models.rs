use super::Store;
use crate::error::Result;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::BTreeMap;

pub(crate) type ChatModels = BTreeMap<String, String>;

const MAX_BYTES: usize = 1024 * 1024;

pub(super) fn migrate(db: &Connection) -> Result<()> {
    let mut statement = db.prepare("PRAGMA table_info(chat_preferences)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == "models" {
            return Ok(());
        }
    }
    drop(statement);

    db.execute(
        "ALTER TABLE chat_preferences ADD COLUMN models TEXT NOT NULL DEFAULT '{}'",
        [],
    )?;
    Ok(())
}

impl Store {
    pub(crate) fn chat_models(&self, chat: i64) -> Result<ChatModels> {
        let bytes: Option<Vec<u8>> = self
            .db
            .query_row(
                "SELECT substr(cast(models AS BLOB),1,?2) FROM chat_preferences WHERE chat=?1",
                params![chat, i64::try_from(MAX_BYTES + 1)?],
                |row| row.get(0),
            )
            .optional()?;
        let Some(bytes) = bytes else {
            return Ok(ChatModels::new());
        };
        if bytes.len() > MAX_BYTES {
            return Err("Saved chat models exceed 1 MiB".into());
        }

        let models = serde_json::from_slice(&bytes)?;
        validate(&models)?;
        Ok(models)
    }

    pub(crate) fn save_chat_models(&self, chat: i64, models: &ChatModels) -> Result<()> {
        validate(models)?;
        let encoded = serde_json::to_string(models)?;
        if encoded.len() > MAX_BYTES {
            return Err("Saved chat models exceed 1 MiB".into());
        }

        self.db.execute(
            "INSERT INTO chat_preferences(chat,models) VALUES(?1,?2)
             ON CONFLICT(chat) DO UPDATE SET models=?2",
            params![chat, encoded],
        )?;
        Ok(())
    }
}

fn validate(models: &ChatModels) -> Result<()> {
    if models.len() > 128
        || models.iter().any(|(agent, model)| {
            agent.is_empty() || agent.len() > 4096 || model.is_empty() || model.len() > 4096
        })
    {
        return Err("Saved chat models exceed their supported limits".into());
    }
    Ok(())
}
