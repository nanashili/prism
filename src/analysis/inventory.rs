use super::Rea;
use crate::error::{Result, failure};
use crate::model::Procedure;
use crate::storage::Store;
use serde_json::{Value, json};

impl Rea {
    pub fn index(&mut self, store: &mut Store, project: &str) -> Result<usize> {
        let reply = self.tool("list_procedures", json!({}))?;
        let normalized = reply.get("normalized_result").unwrap_or(&reply);
        let mut inventory = store.begin_functions(project)?;
        if let Some(rows) = normalized.as_array() {
            for page in rows.chunks(256) {
                inventory.append(&parse(page)?)?;
            }
            return inventory.finish();
        }
        let evidence = reply
            .pointer("/evidence_reference/evidence_id")
            .and_then(Value::as_str)
            .ok_or("REA did not return a function inventory")?;
        let evidence = evidence.to_owned();
        let mut offset = 0;
        loop {
            let page = self.tool(
                "inspect_analysis_view",
                json!({
                    "source":{"kind":"retained-evidence","evidence_id":evidence},
                    "view":{"kind":"page","collection":"procedures","offset":offset,"limit":256}
                }),
            )?;
            let page = page.get("normalized_result").unwrap_or(&page);
            if page["parent_evidence_id"] != evidence {
                return Err(failure("The function page belongs to a different analysis"));
            }
            let rows = page["items"]
                .as_array()
                .ok_or("REA omitted the function page")?;
            inventory.append(&parse(rows)?)?;
            if page.pointer("/coverage/exhausted") == Some(&Value::Bool(true)) {
                break;
            }
            let next = page
                .pointer("/coverage/next_offset")
                .and_then(Value::as_u64)
                .ok_or("Missing next function offset")?;
            if next <= offset {
                return Err(failure("The function inventory did not advance"));
            }
            offset = next;
        }
        inventory.finish()
    }
}

fn parse(rows: &[Value]) -> Result<Vec<Procedure>> {
    rows.iter()
        .map(|row| {
            let address = row["address"]
                .as_str()
                .ok_or("A function is missing its address")?;
            let name = row["value"]
                .as_str()
                .or_else(|| row["name"].as_str())
                .ok_or("A function is missing its name")?;
            let kind = if row.pointer("/procedure/external") == Some(&Value::Bool(true)) {
                "External"
            } else if row.pointer("/procedure/thunk") == Some(&Value::Bool(true)) {
                "Thunk"
            } else {
                "Function"
            };
            Ok(Procedure {
                address: address.into(),
                name: name.into(),
                kind: kind.into(),
            })
        })
        .collect()
}
