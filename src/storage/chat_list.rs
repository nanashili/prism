use super::Store;
use crate::error::Result;
use crate::model::ChatSummary;
use rusqlite::{OptionalExtension, params};

pub(crate) struct StoredChat {
    pub id: i64,
    pub title: String,
    pub project_title: String,
    pub updated: i64,
    pub archived: bool,
}

impl Store {
    pub fn new_chat(&self, project: Option<&str>, title: &str) -> Result<i64> {
        self.db.execute(
            "INSERT INTO chats(project,title,updated) VALUES(?1,?2,unixepoch())",
            params![project, title],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn chat_title(&self, id: i64) -> Result<String> {
        Ok(self
            .db
            .query_row("SELECT title FROM chats WHERE id=?1", [id], |row| {
                row.get(0)
            })?)
    }

    pub fn rename_chat(&self, id: i64, title: &str) -> Result<()> {
        self.db
            .execute("UPDATE chats SET title=?2 WHERE id=?1", params![id, title])?;
        Ok(())
    }

    pub fn delete_chats(&self, ids: &[i64]) -> Result<()> {
        let transaction = self.db.unchecked_transaction()?;
        for id in ids {
            transaction.execute(
                "DELETE FROM message_activity WHERE message IN (SELECT id FROM messages WHERE chat=?1)",
                [id],
            )?;
            transaction.execute("DELETE FROM messages WHERE chat=?1", [id])?;
            transaction.execute("DELETE FROM chat_preferences WHERE chat=?1", [id])?;
            transaction.execute("DELETE FROM chat_selection WHERE chat=?1", [id])?;
            transaction.execute("DELETE FROM chat_drafts WHERE chat=?1", [id])?;
            transaction.execute("DELETE FROM chat_queue WHERE chat=?1", [id])?;
            transaction.execute("DELETE FROM chats WHERE id=?1", [id])?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn clear_chats(&self) -> Result<()> {
        let transaction = self.db.unchecked_transaction()?;
        transaction.execute("DELETE FROM message_activity", [])?;
        transaction.execute("DELETE FROM messages", [])?;
        transaction.execute("DELETE FROM chat_preferences", [])?;
        transaction.execute("DELETE FROM chat_selection", [])?;
        transaction.execute("DELETE FROM chat_drafts", [])?;
        transaction.execute("DELETE FROM chat_queue", [])?;
        transaction.execute("DELETE FROM chats", [])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn chats(&self) -> Result<Vec<ChatSummary>> {
        self.chats_for(None, "", false, 0)
    }

    pub fn chat_count(&self, project: Option<&str>, archived: bool) -> Result<usize> {
        let count: u32 = self.db.query_row(
            "SELECT count(*) FROM chats c LEFT JOIN chat_preferences p ON p.chat=c.id
             WHERE c.project IS ?1 AND coalesce(p.archived,0)=?2",
            params![project, archived],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn chats_for(
        &self,
        project: Option<&str>,
        filter: &str,
        archived: bool,
        offset: usize,
    ) -> Result<Vec<ChatSummary>> {
        self.chat_page(project, filter, archived, offset, true)
    }

    pub fn past_chats(
        &self,
        project: Option<&str>,
        filter: &str,
        offset: usize,
    ) -> Result<Vec<ChatSummary>> {
        self.chat_page(project, filter, false, offset, false)
    }

    pub fn stored_chats(
        &self,
        filter: &str,
        archived: bool,
        offset: usize,
    ) -> Result<Vec<StoredChat>> {
        let mut statement = self.db.prepare(
            "SELECT c.id,substr(coalesce(c.title,'Untitled chat'),1,200),c.updated,
                    substr(coalesce(r.title,CASE WHEN c.project IS NULL THEN 'General chats'
                           ELSE 'Unavailable project' END),1,160),coalesce(p.archived,0)
             FROM chats c LEFT JOIN chat_preferences p ON p.chat=c.id
             LEFT JOIN projects r ON r.id=c.project
             WHERE coalesce(p.archived,0)=?1 AND instr(lower(coalesce(c.title,'')),lower(?2))>0
             ORDER BY c.updated ASC,c.id ASC LIMIT 51 OFFSET ?3",
        )?;
        let rows =
            statement.query_map(params![archived, filter, i64::try_from(offset)?], |row| {
                Ok(StoredChat {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    updated: row.get(2)?,
                    project_title: row.get(3)?,
                    archived: row.get(4)?,
                })
            })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    fn chat_page(
        &self,
        project: Option<&str>,
        filter: &str,
        archived: bool,
        offset: usize,
        pinned_first: bool,
    ) -> Result<Vec<ChatSummary>> {
        let mut statement = self.db.prepare(
            "SELECT c.id,c.title,c.updated,c.project,coalesce(p.pinned,0),coalesce(p.archived,0)
             FROM chats c LEFT JOIN chat_preferences p ON p.chat=c.id
             WHERE c.project IS ?1 AND coalesce(p.archived,0)=?2
             AND instr(lower(c.title),lower(?3))>0
             ORDER BY CASE WHEN ?5 THEN coalesce(p.pinned,0) ELSE 0 END DESC,
             c.updated DESC,c.id DESC LIMIT 51 OFFSET ?4",
        )?;
        let rows = statement.query_map(
            params![
                project,
                archived,
                filter,
                i64::try_from(offset)?,
                pinned_first
            ],
            |row| {
                Ok(ChatSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    updated: row.get(2)?,
                    project: row.get(3)?,
                    pinned: row.get(4)?,
                    archived: row.get(5)?,
                })
            },
        )?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn pin_chat(&self, id: i64, pinned: bool) -> Result<()> {
        self.db.execute(
            "INSERT INTO chat_preferences(chat,pinned) VALUES(?1,?2)
             ON CONFLICT(chat) DO UPDATE SET pinned=?2",
            params![id, pinned],
        )?;
        Ok(())
    }

    pub fn archive_chat(&self, id: i64, archived: bool) -> Result<()> {
        self.db.execute(
            "INSERT INTO chat_preferences(chat,archived) VALUES(?1,?2)
             ON CONFLICT(chat) DO UPDATE SET archived=?2",
            params![id, archived],
        )?;
        Ok(())
    }

    pub fn remember_chat(&self, project: Option<&str>, chat: Option<i64>) -> Result<()> {
        self.db.execute(
            "INSERT INTO chat_selection(project,chat) VALUES(?1,?2)
             ON CONFLICT(project) DO UPDATE SET chat=?2",
            params![project.unwrap_or(""), chat],
        )?;
        Ok(())
    }

    pub fn selected_chat(&self, project: Option<&str>) -> Result<Option<i64>> {
        Ok(self
            .db
            .query_row(
                "SELECT c.id FROM chat_selection s JOIN chats c ON c.id=s.chat
             LEFT JOIN chat_preferences p ON p.chat=c.id
             WHERE s.project=?1 AND c.project IS ?2 AND coalesce(p.archived,0)=0",
                params![project.unwrap_or(""), project],
                |row| row.get(0),
            )
            .optional()?)
    }
}
