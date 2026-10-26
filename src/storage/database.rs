use crate::error::Result;
use crate::model::{Procedure, Project, Recent};
use rusqlite::{Connection, OpenFlags, params};
use std::path::Path;

pub(crate) struct Store {
    pub(super) db: Connection,
    pub(super) cache: Connection,
}

impl Store {
    pub(crate) fn open_readonly(path: &Path) -> Result<Self> {
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        db.busy_timeout(std::time::Duration::from_millis(250))?;
        db.execute_batch("PRAGMA query_only=ON; PRAGMA cache_size=-1024;")?;
        let cache = Connection::open_in_memory()?;
        Ok(Self { db, cache })
    }

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(3))?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA cache_size=-2048;
             CREATE TABLE IF NOT EXISTS projects(
                 id TEXT PRIMARY KEY, title TEXT, path TEXT, state TEXT, opened INTEGER);
             CREATE TABLE IF NOT EXISTS functions(
                 project TEXT, address TEXT, name TEXT, kind TEXT,
                 PRIMARY KEY(project,address));
             CREATE INDEX IF NOT EXISTS function_name ON functions(project,name,address);
             CREATE TABLE IF NOT EXISTS function_sources(
                 project TEXT, address TEXT, path TEXT, owner TEXT, signature TEXT,
                 PRIMARY KEY(project,address));
             CREATE TABLE IF NOT EXISTS analysis_runs(
                 project TEXT PRIMARY KEY, completed INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS chats(
                 id INTEGER PRIMARY KEY, project TEXT, title TEXT, updated INTEGER);
             CREATE TABLE IF NOT EXISTS messages(
                 id INTEGER PRIMARY KEY, chat INTEGER, role TEXT, text TEXT);
             CREATE INDEX IF NOT EXISTS message_chat ON messages(chat,id);
             CREATE INDEX IF NOT EXISTS chat_project_updated ON chats(project,updated DESC);
             CREATE TABLE IF NOT EXISTS chat_preferences(
                 chat INTEGER PRIMARY KEY, pinned INTEGER NOT NULL DEFAULT 0,
                 archived INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE IF NOT EXISTS chat_selection(project TEXT PRIMARY KEY,chat INTEGER);
             CREATE TABLE IF NOT EXISTS chat_drafts(
                 chat INTEGER PRIMARY KEY,draft TEXT NOT NULL,
                 attachments TEXT NOT NULL DEFAULT '[]');
             CREATE TABLE IF NOT EXISTS chat_queue(
                 chat INTEGER PRIMARY KEY, data TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS message_activity(
                 message INTEGER PRIMARY KEY, data TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS views(
                 project TEXT, address TEXT, kind TEXT, data BLOB, accessed INTEGER,
                 PRIMARY KEY(project,address,kind));
             CREATE TABLE IF NOT EXISTS decompilation_jobs(
                 id TEXT PRIMARY KEY,
                 project TEXT NOT NULL,
                 provider TEXT NOT NULL,
                 selection TEXT NOT NULL,
                 options TEXT NOT NULL,
                 state TEXT NOT NULL,
                 total INTEGER NOT NULL,
                 completed INTEGER NOT NULL DEFAULT 0,
                 failed INTEGER NOT NULL DEFAULT 0,
                 next_offset INTEGER NOT NULL DEFAULT 0,
                 artifact_bytes INTEGER NOT NULL DEFAULT 0,
                 created INTEGER NOT NULL,
                 updated INTEGER NOT NULL,
                 error TEXT);
             CREATE TABLE IF NOT EXISTS decompilation_results(
                 job TEXT NOT NULL REFERENCES decompilation_jobs(id) ON DELETE CASCADE,
                 address TEXT NOT NULL,
                 status TEXT NOT NULL,
                 data BLOB,
                 digest TEXT,
                 error TEXT,
                 error_details TEXT,
                 attempts INTEGER NOT NULL DEFAULT 1,
                 updated INTEGER NOT NULL,
                 PRIMARY KEY(job,address));
             CREATE TABLE IF NOT EXISTS decompilation_items(
                 job TEXT NOT NULL REFERENCES decompilation_jobs(id) ON DELETE CASCADE,
                 position INTEGER NOT NULL,
                 address TEXT NOT NULL,
                 module TEXT NOT NULL DEFAULT '',
                 PRIMARY KEY(job,position),
                 UNIQUE(job,address));
             CREATE INDEX IF NOT EXISTS decompilation_results_job_status
                 ON decompilation_results(job,status,address);
             CREATE INDEX IF NOT EXISTS decompilation_results_address_job
                 ON decompilation_results(address,job);
             CREATE INDEX IF NOT EXISTS decompilation_jobs_project_provider
                 ON decompilation_jobs(project,provider);",
        )?;
        super::database_migrations::migrate_decompilation_schema(&db)?;
        migrate_draft_attachments(&db)?;
        super::chat_models::migrate(&db)?;
        super::execution::migrate(&db)?;
        let cache = super::cache::open(&path.with_extension("views.db"))?;
        Ok(Self { db, cache })
    }

    pub fn save_project(&self, project: &Project) -> Result<()> {
        self.db.execute(
            "INSERT INTO projects VALUES(?1,?2,?3,?4,unixepoch())
             ON CONFLICT(id) DO UPDATE SET title=?2,path=?3,state=?4,opened=unixepoch()",
            params![
                project.target.sha256,
                project.target.name,
                project
                    .source_document
                    .as_ref()
                    .unwrap_or(&project.target.path)
                    .to_string_lossy(),
                serde_json::to_string(project)?
            ],
        )?;
        Ok(())
    }

    pub fn project(&self, id: &str) -> Result<Project> {
        let state: String =
            self.db
                .query_row("SELECT state FROM projects WHERE id=?1", [id], |row| {
                    row.get(0)
                })?;
        Ok(serde_json::from_str(&state)?)
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<Recent>> {
        let mut statement = self.db.prepare(
            "SELECT id,title,path,opened,
                (SELECT count(*) FROM chats WHERE project=projects.id),
                coalesce(json_extract(state,'$.target.original_path'),
                         json_extract(state,'$.target.path'),path)
             FROM projects ORDER BY opened DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([i64::try_from(limit)?], |row| {
            Ok(Recent {
                opened: row.get(3)?,
                chats: row.get(4)?,
                id: row.get(0)?,
                title: row.get(1)?,
                path: row.get::<_, String>(2)?.into(),
                original: row.get::<_, String>(5)?.into(),
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn functions(&self, project: &str, filter: &str, offset: usize) -> Result<Vec<Procedure>> {
        let mut statement = self.db.prepare(
            "SELECT address,name,kind FROM functions
             WHERE project=?1 AND (instr(lower(name),lower(?2))>0 OR instr(address,?2)>0)
             ORDER BY address LIMIT 300 OFFSET ?3",
        )?;
        let rows =
            statement.query_map(params![project, filter, i64::try_from(offset)?], |row| {
                Ok(Procedure {
                    address: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                })
            })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

fn migrate_draft_attachments(db: &Connection) -> Result<()> {
    let mut statement = db.prepare("PRAGMA table_info(chat_drafts)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == "attachments" {
            return Ok(());
        }
    }
    drop(statement);
    db.execute(
        "ALTER TABLE chat_drafts ADD COLUMN attachments TEXT NOT NULL DEFAULT '[]'",
        [],
    )?;
    Ok(())
}
