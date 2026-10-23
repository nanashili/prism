use super::Store;
use crate::error::{Result, failure};
use crate::model::{Message, Procedure, Project};
use rusqlite::{OptionalExtension, params};

impl Store {
    pub(crate) fn prepare_upstream_import(&self) -> Result<()> {
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS upstream_threads(
                original TEXT PRIMARY KEY, chat INTEGER NOT NULL);",
        )?;
        Ok(())
    }

    pub(crate) fn import_project(&self, project: &Project, opened: i64) -> Result<bool> {
        let changed = self.db.execute(
            "INSERT OR IGNORE INTO projects VALUES(?1,?2,?3,?4,?5)",
            params![
                project.target.sha256,
                project.target.name,
                project.target.path.to_string_lossy(),
                serde_json::to_string(project)?,
                opened / 1000,
            ],
        )?;
        Ok(changed == 1)
    }

    pub(crate) fn import_thread(
        &self,
        original: &str,
        project: Option<&str>,
        title: &str,
        updated: i64,
        messages: &[Message],
    ) -> Result<bool> {
        let transaction = self.db.unchecked_transaction()?;
        let existing: Option<i64> = transaction
            .query_row(
                "SELECT chat FROM upstream_threads WHERE original=?1",
                [original],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(chat) = existing {
            restore_activities(&transaction, chat, messages)?;
            transaction.commit()?;
            return Ok(false);
        }
        transaction.execute(
            "INSERT INTO chats(project,title,updated) VALUES(?1,?2,?3)",
            params![project, title, updated / 1000],
        )?;
        let chat = transaction.last_insert_rowid();
        {
            let mut insert =
                transaction.prepare("INSERT INTO messages(chat,role,text) VALUES(?1,?2,?3)")?;
            for message in messages {
                insert.execute(params![chat, message.role, message.text])?;
            }
        }
        restore_activities(&transaction, chat, messages)?;
        transaction.execute(
            "INSERT INTO upstream_threads VALUES(?1,?2)",
            params![original, chat],
        )?;
        transaction.commit()?;
        Ok(true)
    }

    pub(crate) fn import_procedures(&self, project: &str, text: &str) -> Result<usize> {
        let transaction = self.db.unchecked_transaction()?;
        let mut added = 0;
        {
            let mut insert =
                transaction.prepare("INSERT OR IGNORE INTO functions VALUES(?1,?2,?3,?4)")?;
            crate::protocol::json_array::parse(text, |row: Procedure| {
                added += insert.execute(params![project, row.address, row.name, row.kind])?;
                Ok(())
            })?;
        }
        transaction.commit()?;
        Ok(added)
    }

    pub(crate) fn import_view(
        &self,
        project: &str,
        address: &str,
        kind: &str,
        text: &str,
    ) -> Result<bool> {
        if text.len() > 2 * 1024 * 1024 {
            return Err(failure("The imported analysis view exceeds 2 MiB"));
        }
        let compressed = zstd::encode_all(text.as_bytes(), 3)?;
        let added = self.cache.execute(
            "INSERT OR IGNORE INTO views VALUES(?1,?2,?3,?4,unixepoch())",
            params![project, address, format!("upstream:{kind}"), compressed],
        )?;
        Ok(added == 1)
    }

    pub(crate) fn verify_import(&self) -> Result<()> {
        for database in [&self.db, &self.cache] {
            let result: String = database.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
            if result != "ok" {
                return Err(failure(format!(
                    "Imported database failed verification: {result}"
                )));
            }
        }
        Ok(())
    }
}

fn restore_activities(
    transaction: &rusqlite::Transaction<'_>,
    chat: i64,
    messages: &[Message],
) -> Result<()> {
    let mut previous = 0;
    for message in messages {
        let id: Option<i64> = transaction
            .query_row(
                "SELECT id FROM messages
                 WHERE chat=?1 AND id>?2 AND role=?3 AND text=?4
                 ORDER BY id LIMIT 1",
                params![chat, previous, message.role, message.text],
                |row| row.get(0),
            )
            .optional()?;
        let Some(id) = id else {
            continue;
        };
        previous = id;
        if let Some(activity) = &message.activity {
            transaction.execute(
                "INSERT OR IGNORE INTO message_activity(message,data) VALUES(?1,?2)",
                params![id, serde_json::to_string(activity)?],
            )?;
        }
    }
    Ok(())
}
