use super::Store;
use crate::error::{Result, failure};
use rusqlite::{Connection, params};

impl Store {
    pub fn reconstruction_inventory(
        &self,
        destination: &Connection,
        project: &str,
        generation: i64,
    ) -> Result<()> {
        let mut query = self.db.prepare(
            "SELECT f.address,f.name,f.kind,coalesce(s.owner,''),coalesce(s.path,'')
             FROM functions f LEFT JOIN function_sources s ON s.project=f.project AND s.address=f.address
             WHERE f.project=?1 ORDER BY f.address",
        )?;
        let mut rows = query.query([project])?;
        let mut insert = destination.prepare(
            "INSERT INTO units(generation,address,name,module,kind) VALUES(?1,?2,?3,?4,?5)",
        )?;
        insert.execute(params![
            generation,
            "__project__",
            "Project structure, resources and integration",
            "",
            "project"
        ])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            count += 1;
            if count > 1_000_000 {
                return Err(failure(
                    "The reconstruction inventory exceeds one million functions",
                ));
            }
            let address: String = row.get(0)?;
            let name: String = row.get(1)?;
            let owner: String = row.get(3)?;
            let path: String = row.get(4)?;
            let module = if !owner.is_empty() {
                owner.as_str()
            } else if !path.is_empty() {
                path.as_str()
            } else {
                name.split_once("::").map_or("Unmapped", |(owner, _)| owner)
            };
            insert.execute(params![
                generation,
                address,
                name,
                module,
                row.get::<_, String>(2)?
            ])?;
        }
        destination.execute(
            "INSERT OR IGNORE INTO modules(generation,module,state)
             SELECT generation,module,'pending' FROM units WHERE generation=?1 GROUP BY generation,module",
            [generation],
        )?;
        Ok(())
    }
}
