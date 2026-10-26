use super::{Show, Sort};
use crate::error::Result;
use crate::storage::Store;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub closed: Vec<String>,
    pub sort: Sort,
    pub show: Show,
    pub hidden: bool,
}

impl Store {
    pub(crate) fn sidebar_preferences(&self, project: &str) -> Result<Preferences> {
        self.prepare_sidebar_preferences()?;
        let state: Option<String> = self
            .db
            .query_row(
                "SELECT state FROM sidebar_preferences WHERE project=?1",
                [project],
                |row| row.get(0),
            )
            .optional()?;
        state.map_or_else(
            || Ok(Preferences::default()),
            |state| Ok(serde_json::from_str(&state)?),
        )
    }

    pub(crate) fn save_sidebar_preferences(
        &self,
        project: &str,
        state: &Preferences,
    ) -> Result<()> {
        self.prepare_sidebar_preferences()?;
        self.db.execute(
            "INSERT INTO sidebar_preferences VALUES(?1,?2)
             ON CONFLICT(project) DO UPDATE SET state=?2",
            params![project, serde_json::to_string(state)?],
        )?;
        Ok(())
    }

    fn prepare_sidebar_preferences(&self) -> Result<()> {
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS sidebar_preferences(project TEXT PRIMARY KEY,state TEXT NOT NULL)",
        )?;
        Ok(())
    }
}
