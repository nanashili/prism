use super::DecompilationResult;
use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::{params, types::Type};
use serde_json::Value;
use sha2::{Digest, Sha256};

impl Store {
    pub(crate) fn decompilation_results_page(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
        include_data: ResultDetail,
    ) -> Result<(Vec<DecompilationResult>, bool)> {
        let include_data = include_data == ResultDetail::WithData;

        let limit = limit.min(100);
        let query_limit = limit.saturating_add(1);
        let byte_limit = 1008 * 1024;
        let mut statement = self.db.prepare(
            "SELECT r.address,coalesce(i.module,''),r.status,
                    CASE WHEN ?4 THEN r.data ELSE NULL END,r.digest,r.error,
                    r.error_details
             FROM decompilation_results r LEFT JOIN decompilation_items i
               ON i.job=r.job AND i.address=r.address
             WHERE r.job=?1 ORDER BY i.position LIMIT ?2 OFFSET ?3",
        )?;
        let rows = statement.query_map(
            params![
                id,
                i64::try_from(query_limit)?,
                i64::try_from(offset)?,
                include_data
            ],
            |row| {
                let compressed: Option<Vec<u8>> = row.get(3)?;
                let digest: Option<String> = row.get(4)?;
                let error_json: Option<String> = row.get(6)?;
                let error_details = error_json
                    .map(|json| {
                        serde_json::from_str::<Value>(&json).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                6,
                                Type::Text,
                                Box::new(error),
                            )
                        })
                    })
                    .transpose()?;
                let status: String = row.get(2)?;
                let data = if include_data {
                    if status == "succeeded" && compressed.is_none() {
                        return Err(rusqlite::Error::ToSqlConversionFailure(failure(
                            "The durable decompilation artifact is missing",
                        )));
                    }
                    compressed
                        .map(|bytes| crate::project::archive::decode(&bytes, 2 * 1024 * 1024))
                        .transpose()
                        .map_err(rusqlite::Error::ToSqlConversionFailure)?
                        .map(|bytes| {
                            let text = String::from_utf8_lossy(&bytes).into_owned();
                            if digest.as_deref()
                                != Some(&format!("{:x}", Sha256::digest(text.as_bytes())))
                            {
                                return Err(rusqlite::Error::ToSqlConversionFailure(failure(
                                    "The durable decompilation artifact failed its digest check",
                                )));
                            }
                            Ok(text)
                        })
                        .transpose()?
                } else {
                    None
                };
                Ok(DecompilationResult {
                    address: row.get(0)?,
                    module: row.get(1)?,
                    status,
                    digest,
                    error: row.get(5)?,
                    error_details,
                    data,
                })
            },
        )?;
        let mut results = Vec::new();
        let mut bytes = 0usize;
        for row in rows {
            let row = row?;
            let size = serde_json::to_vec(&row)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?
                .len();
            if bytes.saturating_add(size) > byte_limit {
                if results.is_empty() {
                    return Err(failure("A stored result exceeds the response size limit"));
                }
                return Ok((results, true));
            }
            bytes = bytes.saturating_add(size);
            results.push(row);
            if results.len() > limit {
                results.pop();
                return Ok((results, true));
            }
        }
        Ok((results, false))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResultDetail {
    WithData,
    Summary,
}

impl From<bool> for ResultDetail {
    fn from(value: bool) -> Self {
        if value { Self::WithData } else { Self::Summary }
    }
}
