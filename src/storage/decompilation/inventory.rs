use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

#[cfg(feature = "rea-rust-ghidra")]
mod window;

impl Store {
    pub(crate) fn decompilation_inventory_digest(
        &self,
        project: &str,
        filter: &str,
        addresses: Option<&[String]>,
    ) -> Result<String> {
        inventory_digest(&self.db, project, filter, addresses)
    }
}

pub(super) fn inventory_digest(
    connection: &Connection,
    project: &str,
    filter: &str,
    addresses: Option<&[String]>,
) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(b"prism-inventory-v1\0");
    digest.update(project.as_bytes());
    digest.update([0]);
    if let Some(addresses) = addresses {
        let mut query = connection.prepare(
            "SELECT f.address,f.name,coalesce(nullif(s.owner,''),nullif(s.path,''),'Unmapped')
                 FROM functions f LEFT JOIN function_sources s
                   ON s.project=f.project AND s.address=f.address
                 WHERE f.project=?1 AND f.address=?2",
        )?;
        for address in addresses {
            let row = query
                .query_row(params![project, address], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .optional()?
                .ok_or_else(|| failure("Every selected address must belong to the project"))?;
            hash_inventory_row(&mut digest, &row.0, &row.1, &row.2);
        }
    } else {
        let mut query = connection.prepare(
            "SELECT f.address,f.name,coalesce(nullif(s.owner,''),nullif(s.path,''),'Unmapped')
                 FROM functions f LEFT JOIN function_sources s
                   ON s.project=f.project AND s.address=f.address
                 WHERE f.project=?1 AND (?2='' OR instr(lower(f.name),lower(?2))>0
                   OR instr(f.address,?2)>0) ORDER BY f.address",
        )?;
        let rows = query.query_map(params![project, filter], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let row = row?;
            hash_inventory_row(&mut digest, &row.0, &row.1, &row.2);
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn hash_inventory_row(digest: &mut Sha256, address: &str, name: &str, module: &str) {
    for value in [address, name, module] {
        digest.update(value.as_bytes());
        digest.update([0]);
    }
}
