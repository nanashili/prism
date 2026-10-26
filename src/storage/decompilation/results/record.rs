use super::quota::{PreviousArtifact, enforce_quota, prepare_artifact};
use crate::error::{Result, failure};
use crate::storage::Store;
use crate::storage::decompilation::DecompilationError;
use rusqlite::{OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

impl Store {
    pub(crate) fn skip_verified_decompilation_result(
        &self,
        id: &str,
        address: &str,
        next_offset: usize,
    ) -> Result<bool> {
        let transaction = self.db.unchecked_transaction()?;
        let previous = load_previous(&transaction, id, address)?;
        if !should_reuse(&previous, None) || verify_previous_success(previous.as_ref()).is_err() {
            transaction.commit()?;
            return Ok(false);
        }

        let expected_offset = validate_cursor(&transaction, id, address, next_offset)?;
        advance_cursor(&transaction, id, next_offset, expected_offset)?;
        transaction.commit()?;
        Ok(true)
    }

    pub(crate) fn record_decompilation_result(
        &self,
        id: &str,
        address: &str,
        status: &str,
        text: Option<&str>,
        error: Option<DecompilationError<'_>>,
        next_offset: usize,
    ) -> Result<bool> {
        let (compressed, digest) = prepare_artifact(text)?;
        let transaction = self.db.unchecked_transaction()?;
        let previous = load_previous(&transaction, id, address)?;
        let expected_offset = validate_cursor(&transaction, id, address, next_offset)?;

        if should_reuse(&previous, text) {
            verify_previous_success(previous.as_ref())?;
            advance_cursor(&transaction, id, next_offset, expected_offset)?;
            transaction.commit()?;
            return Ok(false);
        }

        enforce_quota(
            &transaction,
            id,
            previous.as_ref(),
            compressed.as_deref(),
            error.and_then(|error| error.details),
        )?;
        write_result(
            &transaction,
            id,
            address,
            status,
            compressed.as_deref(),
            digest.as_deref(),
            error,
        )?;
        if let Some(text) = text {
            let project: String = transaction.query_row(
                "SELECT project FROM decompilation_jobs WHERE id=?1",
                [id],
                |row| row.get(0),
            )?;
            crate::storage::verification::observe_pseudocode(
                &transaction,
                &project,
                address,
                text,
            )?;
        }
        update_progress(&transaction, id, previous.as_ref(), status)?;
        advance_cursor(&transaction, id, next_offset, expected_offset)?;
        transaction.commit()?;
        Ok(true)
    }
}

fn load_previous(
    transaction: &Transaction<'_>,
    id: &str,
    address: &str,
) -> Result<Option<PreviousArtifact>> {
    Ok(transaction
        .query_row(
            "SELECT status,data,digest,error_details FROM decompilation_results
             WHERE job=?1 AND address=?2",
            params![id, address],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?)
}

fn validate_cursor(
    transaction: &Transaction<'_>,
    id: &str,
    address: &str,
    next_offset: usize,
) -> Result<i64> {
    let current_offset: i64 = transaction.query_row(
        "SELECT next_offset FROM decompilation_jobs WHERE id=?1",
        [id],
        |row| row.get(0),
    )?;
    let expected_offset = i64::try_from(next_offset)?
        .checked_sub(1)
        .ok_or_else(|| failure("The durable decompilation cursor cannot move backwards"))?;
    if current_offset != expected_offset {
        return Err(failure(
            "The durable decompilation cursor changed concurrently",
        ));
    }
    let expected_address: String = transaction.query_row(
        "SELECT address FROM decompilation_items WHERE job=?1 AND position=?2",
        params![id, expected_offset],
        |row| row.get(0),
    )?;
    if expected_address != address {
        return Err(failure(
            "The durable result address does not match the job cursor",
        ));
    }
    Ok(expected_offset)
}

fn should_reuse(previous: &Option<PreviousArtifact>, text: Option<&str>) -> bool {
    matches!(previous, Some((status, _, _, _)) if status == "succeeded") && text.is_none()
}

fn verify_previous_success(previous: Option<&PreviousArtifact>) -> Result<()> {
    let Some((_, Some(data), Some(expected), _)) = previous else {
        return Err(failure("The durable decompilation artifact is missing"));
    };
    let decoded = crate::project::archive::decode(data, 2 * 1024 * 1024)?;
    let actual = format!("{:x}", Sha256::digest(&decoded));
    if actual != *expected {
        return Err(failure(
            "The durable decompilation artifact failed its digest check",
        ));
    }
    Ok(())
}

fn write_result(
    transaction: &Transaction<'_>,
    id: &str,
    address: &str,
    status: &str,
    data: Option<&[u8]>,
    digest: Option<&str>,
    error: Option<DecompilationError<'_>>,
) -> Result<()> {
    let (error, error_details) = error
        .map(|error| (Some(error.message), error.details))
        .unwrap_or_default();
    transaction.execute(
        "INSERT INTO decompilation_results
         (job,address,status,data,digest,error,error_details,attempts,updated)
         VALUES(?1,?2,?3,?4,?5,?6,?7,1,unixepoch())
         ON CONFLICT(job,address) DO UPDATE SET status=excluded.status,
           data=excluded.data,digest=excluded.digest,error=excluded.error,
           error_details=excluded.error_details,
           attempts=decompilation_results.attempts+1,updated=excluded.updated",
        params![id, address, status, data, digest, error, error_details],
    )?;
    Ok(())
}

fn update_progress(
    transaction: &Transaction<'_>,
    id: &str,
    previous: Option<&PreviousArtifact>,
    status: &str,
) -> Result<()> {
    let completed_delta = i64::from(status == "succeeded");
    let failed_delta = i64::from(status == "failed");
    let (old_completed, old_failed) = previous.map_or((0, 0), |(old_status, _, _, _)| {
        (
            i64::from(old_status == "succeeded"),
            i64::from(old_status == "failed"),
        )
    });
    transaction.execute(
        "UPDATE decompilation_jobs SET completed=completed+?2-?3,
         failed=failed+?4-?5,updated=unixepoch() WHERE id=?1",
        params![id, completed_delta, old_completed, failed_delta, old_failed],
    )?;
    Ok(())
}

fn advance_cursor(
    transaction: &Transaction<'_>,
    id: &str,
    next_offset: usize,
    expected_offset: i64,
) -> Result<()> {
    let advanced = transaction.execute(
        "UPDATE decompilation_jobs SET next_offset=?2,updated=unixepoch()
         WHERE id=?1 AND next_offset=?3",
        params![id, i64::try_from(next_offset)?, expected_offset],
    )?;
    if advanced != 1 {
        return Err(failure(
            "The durable decompilation cursor changed concurrently",
        ));
    }
    Ok(())
}
