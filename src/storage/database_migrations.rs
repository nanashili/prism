use crate::error::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub(super) fn migrate_decompilation_schema(db: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(db, TransactionBehavior::Immediate)?;
    create_migration_table(&transaction)?;
    add_missing_columns(&transaction)?;
    backfill_artifact_bytes(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn create_migration_table(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS prism_decompilation_schema_migrations(
             version INTEGER PRIMARY KEY NOT NULL
         )",
    )?;
    Ok(())
}

fn add_missing_columns(transaction: &Transaction<'_>) -> Result<()> {
    if !has_column(transaction, "decompilation_jobs", "artifact_bytes")? {
        transaction.execute(
            "ALTER TABLE decompilation_jobs ADD COLUMN artifact_bytes INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has_column(transaction, "decompilation_results", "error_details")? {
        transaction.execute(
            "ALTER TABLE decompilation_results ADD COLUMN error_details TEXT",
            [],
        )?;
    }
    Ok(())
}

fn backfill_artifact_bytes(transaction: &Transaction<'_>) -> Result<()> {
    if is_artifact_counter_backfilled(transaction)? {
        return Ok(());
    }

    transaction.execute(
        "UPDATE decompilation_jobs AS job
         SET artifact_bytes=COALESCE(
             (SELECT SUM(COALESCE(length(result.data),0)
                         + COALESCE(length(CAST(result.error_details AS BLOB)),0))
              FROM decompilation_results AS result
              WHERE result.job=job.id),
             0
         )",
        [],
    )?;
    transaction.execute(
        "INSERT INTO prism_decompilation_schema_migrations(version) VALUES(1)",
        [],
    )?;
    Ok(())
}

fn is_artifact_counter_backfilled(transaction: &Transaction<'_>) -> Result<bool> {
    Ok(transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM prism_decompilation_schema_migrations WHERE version=1
         )",
        [],
        |row| row.get(0),
    )?)
}

fn has_column(db: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(names
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|name| name == column))
}
