mod quota;
mod record;

use crate::error::Result;
use crate::storage::Store;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
pub(crate) struct DecompilationResult {
    pub address: String,
    pub module: String,
    pub status: String,
    pub digest: Option<String>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<Value>,
    #[serde(rename = "pseudocode")]
    pub data: Option<String>,
}

pub(super) const MAX_JOB_ARTIFACT_BYTES: i64 = 8 * 1024 * 1024 * 1024;

impl Store {
    pub(crate) fn durable_pseudocode(
        &self,
        project: &str,
        provider: &str,
        configuration: &str,
        address: &str,
    ) -> Result<Option<String>> {
        let row: Option<(Option<Vec<u8>>, Option<String>)> = self
            .db
            .query_row(
                "SELECT r.data,r.digest
                 FROM decompilation_results r
                 JOIN decompilation_jobs j ON j.id=r.job
                 WHERE j.project=?1 AND j.provider=?2 AND j.options=?3
                   AND r.address=?4 AND r.status='succeeded'
                 ORDER BY r.updated DESC,j.updated DESC,j.id DESC LIMIT 1",
                params![project, provider, configuration, address],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((Some(data), Some(expected_digest))) = row else {
            return Ok(None);
        };
        let decoded = match crate::project::archive::decode(&data, 512 * 1024) {
            Ok(decoded) => decoded,
            Err(_) => return Ok(None),
        };
        if format!("{:x}", Sha256::digest(&decoded)) != expected_digest {
            return Ok(None);
        }
        Ok(String::from_utf8(decoded).ok())
    }
}
