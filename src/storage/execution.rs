use super::Store;
use crate::analysis::execution::ExecutionTrace;
use crate::analysis::graph::address_value;
use crate::error::{Result, failure};
use crate::model::Procedure;
use rusqlite::{Connection, params};

/// Keep durable evidence separate from evictable static view caches.
pub(super) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS execution_traces(
             project TEXT NOT NULL,
             id TEXT NOT NULL,
             data TEXT NOT NULL,
             PRIMARY KEY(project,id)
         );",
    )?;
    Ok(())
}

impl Store {
    /// Match exact entry addresses; never attribute an arbitrary PC to the nearest symbol.
    pub(crate) fn executed_functions(
        &self,
        project: &str,
        trace: &ExecutionTrace,
        offset: usize,
    ) -> Result<(Vec<Procedure>, usize)> {
        let addresses = trace.executed_addresses();
        let mut query = self
            .db
            .prepare("SELECT address,name,kind FROM functions WHERE project=?1 ORDER BY address")?;
        let functions = query.query_map([project], |row| {
            Ok(Procedure {
                address: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
            })
        })?;
        let mut matched = 0;
        let mut page = Vec::new();
        for function in functions {
            let function = function?;
            if address_value(&function.address)
                .is_some_and(|address| addresses.contains_key(&address))
            {
                if matched >= offset && page.len() < 300 {
                    page.push(function);
                }
                matched += 1;
            }
        }
        Ok((page, matched))
    }

    pub(crate) fn save_execution_trace(&self, project: &str, trace: &ExecutionTrace) -> Result<()> {
        trace.validate(project)?;
        let data = serde_json::to_string(trace)?;
        let transaction = self.db.unchecked_transaction()?;
        let count: u32 = transaction.query_row(
            "SELECT count(*) FROM execution_traces WHERE project=?1",
            [project],
            |row| row.get(0),
        )?;
        if count >= 16 {
            return Err(failure(
                "The project already has 16 traces; remove a trace before importing",
            ));
        }
        // Trace IDs are immutable: retries cannot silently double-count or replace evidence.
        transaction.execute(
            "INSERT INTO execution_traces(project,id,data) VALUES(?1,?2,?3)",
            params![project, trace.id, data],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn execution_traces(&self, project: &str) -> Result<Vec<ExecutionTrace>> {
        let mut query = self
            .db
            .prepare("SELECT data FROM execution_traces WHERE project=?1 ORDER BY id LIMIT 16")?;
        let rows = query.query_map([project], |row| row.get::<_, String>(0))?;
        let mut traces = Vec::new();
        for row in rows {
            let trace: ExecutionTrace = serde_json::from_str(&row?)?;
            trace.validate(project)?;
            traces.push(trace);
        }
        Ok(traces)
    }

    pub(crate) fn remove_execution_trace(&self, project: &str, id: &str) -> Result<()> {
        let removed = self.db.execute(
            "DELETE FROM execution_traces WHERE project=?1 AND id=?2",
            params![project, id],
        )?;
        if removed == 0 {
            return Err(failure(format!("Unknown trace: {id}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn traces_survive_reopening_and_failed_imports_preserve_saved_evidence() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("prism.db");
        let project = "a".repeat(64);
        let mut trace: ExecutionTrace = serde_json::from_value(json!({
            "version":1,"id":"workflow-1","workflow":"open document","source":"imported",
            "runtime_version":"fixture","captured_at":"2026-10-26T00:00:00Z",
            "target_sha256":project,"complete":true,
            "mapping":{"runtime_base":"0x7000","static_base":"0x1000","size":4096},
            "events":[
                {"kind":"call","address":"0x7000","target":"0x7010","count":2,"thread_id":1},
                {"kind":"sample","address":"0x7020","count":1,"thread_id":1}
            ]
        }))?;
        {
            let store = Store::open(&path)?;
            store.save_execution_trace(&project, &trace)?;
            assert!(
                store.save_execution_trace(&project, &trace).is_err(),
                "Duplicate IDs cannot replace evidence"
            );
            trace.id = "bad-mapping".into();
            trace.mapping.runtime_base = "0xfffffffffffffff0".into();
            assert!(
                store.save_execution_trace(&project, &trace).is_err(),
                "Overflow must fail before persistence"
            );
            trace.mapping.runtime_base = "0x7000".into();
            trace.target_sha256 = "b".repeat(64);
            assert!(
                store.save_execution_trace(&project, &trace).is_err(),
                "Another target's observations cannot be imported"
            );
        }

        let mut store = Store::open(&path)?;
        let traces = store.execution_traces(&project)?;
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].id, "workflow-1");
        assert_eq!(
            traces[0].events[0].count, 2,
            "Rejected writes must retain the original observation count"
        );
        assert!(store.execution_traces(&"b".repeat(64))?.is_empty());
        let mut inventory = store.begin_functions(&project)?;
        inventory.append(&[
            Procedure {
                address: "0x1000".into(),
                name: "caller".into(),
                kind: "function".into(),
            },
            Procedure {
                address: "00001010".into(),
                name: "callee".into(),
                kind: "function".into(),
            },
            Procedure {
                address: "0x1020".into(),
                name: "sampled".into(),
                kind: "function".into(),
            },
        ])?;
        inventory.finish()?;
        let (functions, total) = store.executed_functions(&project, &traces[0], 0)?;
        assert_eq!(
            total, 2,
            "A stopped PC sample must not establish function entry execution"
        );
        let names: Vec<_> = functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert!(
            names.contains(&"caller") && names.contains(&"callee"),
            "Relocated entries match even when the provider pads hexadecimal addresses"
        );
        store.remove_execution_trace(&project, "workflow-1")?;
        assert!(store.execution_traces(&project)?.is_empty());
        Ok(())
    }
}
