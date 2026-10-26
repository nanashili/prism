use super::Store;
use crate::error::{Result, failure};
use crate::investigation::Graph;
use rusqlite::{OptionalExtension, params};

pub(super) fn migrate(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS investigations(
            id TEXT PRIMARY KEY, project TEXT NOT NULL, revision INTEGER NOT NULL,
            data TEXT NOT NULL, updated INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS investigations_project_updated
            ON investigations(project,updated DESC,id);",
    )?;
    Ok(())
}

impl Store {
    pub(crate) fn insert_investigation(&self, graph: &Graph) -> Result<()> {
        self.db.execute(
            "INSERT INTO investigations(id,project,revision,data,updated) VALUES(?1,?2,?3,?4,unixepoch('subsec'))",
            params![graph.id, graph.project, graph.revision, serde_json::to_string(graph)?],
        )?;
        Ok(())
    }

    pub(crate) fn investigation(&self, project: &str, id: &str) -> Result<Graph> {
        let data: String = self.db.query_row(
            "SELECT data FROM investigations WHERE project=?1 AND id=?2",
            params![project, id],
            |row| row.get(0),
        )?;
        decode(&data)
    }

    pub(crate) fn investigations(&self, project: &str, offset: u32) -> Result<Vec<Graph>> {
        let mut statement = self.db.prepare(
            "SELECT data FROM investigations WHERE project=?1 ORDER BY updated DESC,rowid DESC LIMIT 20 OFFSET ?2",
        )?;
        let rows = statement.query_map(params![project, offset], |row| row.get::<_, String>(0))?;
        rows.map(|row| decode(&row?)).collect()
    }

    pub(crate) fn latest_investigation(&self, project: &str) -> Result<Option<Graph>> {
        let data: Option<String> = self.db.query_row(
            "SELECT data FROM investigations WHERE project=?1 ORDER BY updated DESC,rowid DESC LIMIT 1",
            [project], |row| row.get(0),
        ).optional()?;
        data.map(|data| decode(&data)).transpose()
    }

    /// Compare-and-swap prevents a stale agent or another connection losing saved work.
    pub(crate) fn save_investigation(&self, graph: &mut Graph) -> Result<()> {
        let revision = graph
            .revision
            .checked_add(1)
            .ok_or("Investigation revision overflowed")?;
        let previous = graph.revision;
        let data = serde_json::to_string(graph)?;
        let changed = self.db.execute(
            "UPDATE investigations SET data=json_set(?1,'$.revision',?2),revision=?2,updated=unixepoch('subsec') WHERE project=?3 AND id=?4 AND revision=?5",
            params![data, revision, graph.project, graph.id, previous],
        )?;
        if changed != 1 {
            return Err(failure(
                "The investigation changed; reload it before retrying this update",
            ));
        }
        graph.revision = revision;
        Ok(())
    }

    pub(crate) fn require_investigation_function(
        &self,
        project: &str,
        address: &str,
    ) -> Result<()> {
        if !self.has_function(project, address)? {
            return Err(failure(format!(
                "Function {address} is not in this project's saved inventory; record the missing evidence as a blocker"
            )));
        }
        Ok(())
    }
}

fn decode(data: &str) -> Result<Graph> {
    let graph: Graph = serde_json::from_str(data)?;
    if graph.version != 1 {
        return Err(failure("Unsupported investigation graph version"));
    }
    Ok(graph)
}
