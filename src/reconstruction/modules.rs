use super::database::Database;
use crate::error::{Result, failure};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const MAX_GRAPH_NODES: usize = 4_096;
const MAX_GRAPH_EDGES: usize = 16_384;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency {
    module: String,
    depends_on: String,
}

impl Database {
    pub fn set_dependencies(&mut self, generation: i64, payload: &Value) -> Result<Value> {
        let raw = payload
            .get("dependencies")
            .and_then(Value::as_array)
            .ok_or("Supply a dependencies array")?;
        if raw.len() > MAX_GRAPH_EDGES {
            return Err(failure("A dependency update is limited to 16,384 edges"));
        }
        let dependencies: Vec<Dependency> = serde_json::from_value(Value::Array(raw.clone()))?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        super::queue::active(&tx, generation)?;
        refuse_leased(&tx, generation)?;
        let graph = validate_graph(&tx, generation, &dependencies)?;
        let new_edges = dependencies
            .iter()
            .map(|edge| (edge.module.clone(), edge.depends_on.clone()))
            .collect::<std::collections::BTreeSet<_>>();
        if new_edges.len() != dependencies.len() {
            return Err(failure(
                "A dependency update cannot contain duplicate edges",
            ));
        }
        let mut old_edges_set = std::collections::BTreeSet::new();
        let mut old =
            tx.prepare("SELECT module,depends_on FROM module_dependencies WHERE generation=?1")?;
        let old_edges = old.query_map([generation], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for edge in old_edges {
            old_edges_set.insert(edge?);
        }
        drop(old);
        let mut changed_modules = std::collections::BTreeSet::new();
        for edge in new_edges.symmetric_difference(&old_edges_set) {
            changed_modules.insert(edge.0.clone());
            changed_modules.insert(edge.1.clone());
        }
        let changed_modules = changed_modules.into_iter().collect::<Vec<_>>();
        let changed = !changed_modules.is_empty();
        tx.execute(
            "DELETE FROM module_dependencies WHERE generation=?1",
            [generation],
        )?;
        tx.execute(
            "UPDATE modules SET cycle_group='' WHERE generation=?1",
            [generation],
        )?;
        insert_dependencies(&tx, generation, &dependencies)?;
        let cycles = super::scc::components(&graph);
        apply_cycle_groups(&tx, generation, &cycles, &graph)?;
        invalidate_consumers(&tx, generation, &changed_modules)?;
        if changed {
            tx.execute(
                "UPDATE generations SET revision=revision+1 WHERE id=?1",
                [generation],
            )?;
            tx.execute("DELETE FROM actions WHERE generation=?1 AND address='__generation__' AND stage='validate'", [generation])?;
        }
        tx.commit()?;

        Ok(json!({"saved": true, "edges": dependencies.len(),
            "modules": graph.len(), "cycles": cycles.len(), "behavior_verified": false}))
    }
}

pub(super) fn invalidate_consumers(
    tx: &Transaction<'_>,
    generation: i64,
    modules: &[String],
) -> Result<()> {
    if modules.is_empty() {
        return Ok(());
    }
    // Traverse the union of affected modules once per statement. Starting a
    // traversal for every changed root repeats work for shared consumers.
    let roots = serde_json::to_string(modules)?;
    tx.execute(
        "UPDATE units SET state='pending',lease=NULL WHERE generation=?1 AND module IN
         (WITH RECURSIVE affected(module) AS (
            SELECT value FROM json_each(?2) UNION SELECT d.module FROM module_dependencies d
            JOIN affected a ON d.depends_on=a.module WHERE d.generation=?1
         ) SELECT module FROM affected) AND state IN ('generated','reused','blocked')",
        params![generation, roots],
    )?;
    tx.execute(
        "UPDATE modules SET state='pending' WHERE generation=?1 AND module IN
         (WITH RECURSIVE affected(module) AS (
            SELECT value FROM json_each(?2) UNION SELECT d.module FROM module_dependencies d
            JOIN affected a ON d.depends_on=a.module WHERE d.generation=?1
         ) SELECT module FROM affected)",
        params![generation, roots],
    )?;
    tx.execute(
        "DELETE FROM actions WHERE generation=?1 AND stage IN ('contract','emit','validate')
         AND address IN (SELECT address FROM units WHERE generation=?1 AND module IN
           (WITH RECURSIVE affected(module) AS (
              SELECT value FROM json_each(?2) UNION SELECT d.module FROM module_dependencies d
              JOIN affected a ON d.depends_on=a.module WHERE d.generation=?1
           ) SELECT module FROM affected))",
        params![generation, roots],
    )?;
    Ok(())
}

pub(super) fn refuse_leased(tx: &Transaction<'_>, generation: i64) -> Result<()> {
    let leased: i64 = tx.query_row(
        "SELECT count(*) FROM units WHERE generation=?1 AND state='leased'",
        [generation],
        |row| row.get(0),
    )?;
    if leased != 0 {
        return Err(failure(
            "Release active reconstruction batches before changing module contracts",
        ));
    }
    Ok(())
}

fn validate_graph(
    tx: &Transaction<'_>,
    generation: i64,
    edges: &[Dependency],
) -> Result<BTreeMap<String, Vec<String>>> {
    let mut graph = BTreeMap::<String, Vec<String>>::new();
    for edge in edges {
        if edge.module.is_empty()
            || edge.depends_on.is_empty()
            || edge.module.len() > 256
            || edge.depends_on.len() > 256
        {
            return Err(failure("Module names must be 1–256 bytes"));
        }
        ensure_module(tx, generation, &edge.module)?;
        ensure_module(tx, generation, &edge.depends_on)?;
        graph
            .entry(edge.module.clone())
            .or_default()
            .push(edge.depends_on.clone());
        graph.entry(edge.depends_on.clone()).or_default();
    }
    if graph.len() > MAX_GRAPH_NODES {
        return Err(failure("The dependency graph is limited to 4,096 modules"));
    }
    Ok(graph)
}

pub(super) fn ensure_module(tx: &Transaction<'_>, generation: i64, module: &str) -> Result<()> {
    let found: Option<i64> = tx
        .query_row(
            "SELECT 1 FROM modules WHERE generation=?1 AND module=?2",
            params![generation, module],
            |row| row.get(0),
        )
        .optional()?;
    if found.is_none() {
        return Err(failure("Dependencies must reference known modules"));
    }
    Ok(())
}

fn insert_dependencies(tx: &Transaction<'_>, generation: i64, edges: &[Dependency]) -> Result<()> {
    for edge in edges {
        tx.execute(
            "INSERT INTO module_dependencies(generation,module,depends_on) VALUES(?1,?2,?3)",
            params![generation, edge.module, edge.depends_on],
        )?;
    }
    Ok(())
}

fn apply_cycle_groups(
    tx: &Transaction<'_>,
    generation: i64,
    cycles: &[Vec<String>],
    graph: &BTreeMap<String, Vec<String>>,
) -> Result<()> {
    for members in cycles.iter().filter(|members| {
        members.len() > 1
            || graph
                .get(&members[0])
                .is_some_and(|edges| edges.contains(&members[0]))
    }) {
        let mut digest = Sha256::new();
        for member in members {
            digest.update(member.as_bytes());
            digest.update([0]);
        }
        let group = format!("scc-{:x}", digest.finalize());
        for member in members {
            tx.execute(
                "UPDATE modules SET cycle_group=?1 WHERE generation=?2 AND module=?3",
                params![group, generation, member],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
