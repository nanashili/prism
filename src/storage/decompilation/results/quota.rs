use super::MAX_JOB_ARTIFACT_BYTES;
use crate::error::{Result, failure};
use rusqlite::{Transaction, params};
use sha2::{Digest, Sha256};

pub(super) type PreviousArtifact = (String, Option<Vec<u8>>, Option<String>, Option<String>);

pub(super) fn prepare_artifact(text: Option<&str>) -> Result<(Option<Vec<u8>>, Option<String>)> {
    let compressed = text
        .map(|value| zstd::encode_all(value.as_bytes(), 3))
        .transpose()?;
    if compressed
        .as_ref()
        .is_some_and(|data| data.len() > 2 * 1024 * 1024)
    {
        return Err(failure(
            "The durable decompilation artifact exceeds the 2 MiB limit",
        ));
    }

    let digest = text.map(|value| format!("{:x}", Sha256::digest(value.as_bytes())));
    Ok((compressed, digest))
}

pub(super) fn enforce_quota(
    transaction: &Transaction<'_>,
    id: &str,
    previous: Option<&PreviousArtifact>,
    compressed: Option<&[u8]>,
    error_details: Option<&str>,
) -> Result<()> {
    let previous_bytes = previous.map_or(0, |(_, data, _, details)| {
        data.as_ref()
            .map_or(0, Vec::len)
            .saturating_add(details.as_ref().map_or(0, String::len))
    });
    let next_bytes = compressed
        .map_or(0, <[u8]>::len)
        .saturating_add(error_details.map_or(0, str::len));
    let delta = i64::try_from(next_bytes)?.saturating_sub(i64::try_from(previous_bytes)?);
    let job_bytes: i64 = transaction.query_row(
        "SELECT artifact_bytes FROM decompilation_jobs WHERE id=?1",
        [id],
        |row| row.get(0),
    )?;
    if job_bytes.saturating_add(delta) > MAX_JOB_ARTIFACT_BYTES {
        return Err(failure("The durable decompilation artifact quota is full"));
    }

    transaction.execute(
        "UPDATE decompilation_jobs SET artifact_bytes=artifact_bytes+?2 WHERE id=?1",
        params![id, delta],
    )?;
    Ok(())
}
