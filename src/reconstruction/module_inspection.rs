use super::database::Database;
use crate::error::{Result, failure};
use rusqlite::{Error as SqlError, params, types::Type};
use serde_json::{Value, json};

impl Database {
    pub fn modules(&self, generation: i64, offset: u64) -> Result<Value> {
        let mut query = self.db.prepare(
            "SELECT m.module,m.state,m.cycle_group,m.contract_ready,
                COALESCE(c.interface,''),COALESCE(c.abi,''),COALESCE(c.unknowns,''),
                COALESCE((SELECT json_group_array(depends_on) FROM module_dependencies
                    WHERE generation=m.generation AND module=m.module),'[]')
             FROM modules m LEFT JOIN contracts c ON c.generation=m.generation AND c.module=m.module
             WHERE m.generation=?1 ORDER BY m.module LIMIT 17 OFFSET ?2",
        )?;
        let rows = query.query_map(params![generation, i64::try_from(offset)?], |row| {
            let dependencies: String = row.get(7)?;
            let dependencies = serde_json::from_str::<Value>(&dependencies).map_err(|error| {
                SqlError::FromSqlConversionFailure(7, Type::Text, Box::new(error))
            })?;
            Ok(json!({"module": row.get::<_, String>(0)?, "state": row.get::<_, String>(1)?,
                "cycle_group": row.get::<_, String>(2)?, "contract_ready": row.get::<_, i64>(3)? != 0,
                "interface": row.get::<_, String>(4)?, "abi": row.get::<_, String>(5)?,
                "unknowns": row.get::<_, String>(6)?, "depends_on": dependencies}))
        })?;
        let rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut modules = Vec::new();
        let mut bytes = 0usize;
        for module in rows.iter().take(16) {
            let size = serde_json::to_vec(module)?.len();
            if size > 1_008 * 1024 {
                return Err(failure("A single module response exceeds the 1 MiB limit"));
            }
            if bytes + size > 1_008 * 1024 {
                break;
            }
            bytes += size;
            modules.push(module.clone());
        }
        let next_offset = (rows.len() == 17 || modules.len() < rows.len())
            .then_some(offset + modules.len() as u64);
        Ok(json!({"modules": modules, "offset": offset,
            "page_size": modules.len(), "next_offset": next_offset,
            "serialized_bytes": bytes}))
    }
}
