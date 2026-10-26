use super::classify;
use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::{OptionalExtension, params};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

impl Store {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(crate) fn prepare_navigation(
        &mut self,
        project: &str,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.db
            .execute_batch("CREATE TEMP TABLE IF NOT EXISTS sidebar_identity(project TEXT)")?;
        let prepared: Option<String> = self
            .db
            .query_row("SELECT project FROM sidebar_identity LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        if prepared.as_deref() == Some(project) {
            return Ok(());
        }
        let modules = registered_modules(self, project)?;
        let transaction = self.db.transaction()?;
        transaction.execute_batch(
            "DROP TABLE IF EXISTS temp.sidebar_functions;
             CREATE TEMP TABLE sidebar_functions(
                address TEXT, position INTEGER PRIMARY KEY, module TEXT, file TEXT,
                origin TEXT, inferred INTEGER DEFAULT 0);
             CREATE TEMP TABLE IF NOT EXISTS sidebar_renames(address TEXT PRIMARY KEY,name TEXT);
             DELETE FROM sidebar_identity;",
        )?;
        index_functions(&transaction, project, &modules, cancelled)?;
        transaction.execute_batch(
            "CREATE UNIQUE INDEX temp.sidebar_address ON sidebar_functions(address);
             CREATE INDEX temp.sidebar_module ON sidebar_functions(module);
             CREATE INDEX temp.sidebar_file ON sidebar_functions(file);",
        )?;
        transaction.execute("INSERT INTO sidebar_identity VALUES(?1)", [project])?;
        transaction.commit()?;
        Ok(())
    }
}

fn registered_modules(store: &Store, project: &str) -> Result<BTreeSet<String>> {
    let mut query = store
        .db
        .prepare("SELECT name FROM functions WHERE project=?1 AND name LIKE '%RegisterModule_%'")?;
    let names = query.query_map([project], |row| row.get::<_, String>(0))?;
    let mut modules = BTreeSet::new();
    for name in names {
        if let Some(module) = classify::registration(&name?) {
            modules.insert(module.to_owned());
        }
    }
    Ok(modules)
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
fn index_functions(
    db: &rusqlite::Transaction<'_>,
    project: &str,
    modules: &BTreeSet<String>,
    cancelled: &AtomicBool,
) -> Result<()> {
    let mut query = db.prepare(
        "SELECT f.address,f.name,f.kind,coalesce(s.path,''),coalesce(s.owner,'')
         FROM functions f LEFT JOIN function_sources s
         ON s.project=f.project AND s.address=f.address WHERE f.project=?1
         ORDER BY length(f.address),f.address",
    )?;
    let mut rows = query.query([project])?;
    let mut insert = db.prepare("INSERT INTO sidebar_functions VALUES(?1,?2,?3,?4,?5,0)")?;
    let mut previous: Option<(i64, Option<String>)> = None;
    let mut position: i64 = 0;
    while let Some(row) = rows.next()? {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Function grouping cancelled"));
        }
        let address: String = row.get(0)?;
        let name: String = row.get(1)?;
        let kind: String = row.get(2)?;
        let path: String = row.get(3)?;
        let owner: String = row.get(4)?;
        let origin = classify::origin(&name, &kind, &path);
        let marker = if origin == "Imports" {
            classify::ModuleMarker::Unrelated
        } else {
            classify::marker(&name, modules)
        };
        let module = classify::source_module(&path)
            .or(marker.module())
            .unwrap_or("");
        let file = classify::file_group(&name, &path, &owner, origin);
        insert.execute(params![address, position, module, file, origin])?;
        if !matches!(marker, classify::ModuleMarker::Unrelated) {
            let module = marker.module();
            if let Some((start, Some(previous_module))) = &previous
                && module == Some(previous_module.as_str())
            {
                db.execute(
                    "UPDATE sidebar_functions SET module=?1,inferred=1
                     WHERE position>?2 AND position<?3 AND module='' AND origin!='Imports'",
                    params![previous_module, start, position],
                )?;
            }
            previous = Some((position, module.map(str::to_owned)));
        }
        position += 1;
    }
    Ok(())
}
