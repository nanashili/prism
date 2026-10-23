use super::database::Database;
use crate::error::{Result, failure};
use rusqlite::{TransactionBehavior, params};

impl Database {
    pub fn control(
        &mut self,
        generation: i64,
        action: &str,
        lease: Option<&str>,
        address: Option<&str>,
    ) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        super::queue::expire(&tx, generation)?;
        let state: String = tx.query_row(
            "SELECT state FROM generations WHERE id=?1",
            [generation],
            |row| row.get(0),
        )?;
        if state == "frozen" {
            return Err(failure(
                "Previous generations are frozen. Select the latest generation",
            ));
        }
        match action {
            "pause" => {
                tx.execute(
                    "UPDATE generations SET state='paused' WHERE id=?1",
                    [generation],
                )?;
                tx.execute("UPDATE units SET state='pending',lease=NULL WHERE generation=?1 AND state='leased'", [generation])?;
                tx.execute(
                    "UPDATE batches SET state='released' WHERE generation=?1 AND state='leased'",
                    [generation],
                )?;
                tx.execute(
                    "UPDATE modules SET state='pending' WHERE generation=?1 AND state='leased'",
                    [generation],
                )?;
            }
            "resume" => {
                tx.execute(
                    "UPDATE generations SET state='active' WHERE id=?1",
                    [generation],
                )?;
            }
            "retry_blocked" => {
                tx.execute("UPDATE units SET state='pending',lease=NULL,reason='' WHERE generation=?1 AND state='blocked'", [generation])?;
                refresh_modules(&tx, generation)?;
            }
            "reopen" => {
                let address = address.ok_or("Supply the address of the unit to refine")?;
                let changed = tx.execute(
                    "UPDATE units SET state='pending',lease=NULL,reason='' WHERE generation=?1 AND address=?2 AND state<>'leased'",
                    params![generation,address],
                )?;
                if changed == 0 {
                    return Err(failure(
                        "This unit is missing or currently leased. Release its batch first",
                    ));
                }
                refresh_modules(&tx, generation)?;
            }
            "renew" | "release" => {
                let lease = lease.ok_or("Supply the lease token")?;
                let changed = tx.execute(
                    "UPDATE batches SET state=?1,expires=unixepoch()+900 WHERE generation=?2 AND token=?3 AND state='leased'",
                    params![if action == "renew" { "leased" } else { "released" }, generation,lease],
                )?;
                if changed == 0 {
                    return Err(failure("This lease expired or was already released"));
                }
                if action == "release" {
                    tx.execute(
                        "UPDATE units SET state='pending',lease=NULL WHERE lease=?1",
                        [lease],
                    )?;
                    refresh_modules(&tx, generation)?;
                }
            }
            _ => return Err(failure("Unknown reconstruction action")),
        }
        tx.commit()?;
        Ok(())
    }
}

fn refresh_modules(db: &rusqlite::Connection, generation: i64) -> Result<()> {
    db.execute(
        "UPDATE modules SET state=CASE
            WHEN EXISTS(SELECT 1 FROM units u WHERE u.generation=modules.generation
                       AND u.module=modules.module AND u.state IN ('pending','leased')) THEN 'pending'
            WHEN EXISTS(SELECT 1 FROM units u WHERE u.generation=modules.generation
                       AND u.module=modules.module AND u.state='blocked') THEN 'blocked'
            ELSE 'complete' END WHERE generation=?1",
        [generation],
    )?;
    Ok(())
}
