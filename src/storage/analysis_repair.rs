use crate::analysis_repair::{Observation, Plan};
use crate::decompilation::Context;
use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_CAPTURE_BYTES: usize = 768 * 1024;
const MAX_PROJECT_BYTES: i64 = 32 * 1024 * 1024;
const MAX_ATTEMPTS: i64 = 2;
const MAX_PLAN_BYTES: i64 = 64 * 1024;
const RESERVED_OVERLAY_BYTES: i64 = (MAX_CAPTURE_BYTES as i64) * MAX_ATTEMPTS + MAX_PLAN_BYTES;

#[cfg(test)]
mod tests;

pub(crate) fn initialize(db: &Connection) -> Result<()> {
    create_tables(db)?;

    let has_reservation = db
        .prepare("PRAGMA table_info(analysis_repairs)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == "reserved_attempt_bytes");
    if !has_reservation {
        db.execute(
            "ALTER TABLE analysis_repairs ADD COLUMN reserved_attempt_bytes INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

fn create_tables(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS analysis_repairs(
             id TEXT PRIMARY KEY, project TEXT NOT NULL, provider TEXT NOT NULL,
             configuration TEXT NOT NULL, baseline TEXT NOT NULL, digest TEXT NOT NULL,
             plan TEXT, state TEXT NOT NULL, created INTEGER NOT NULL,
             reserved_attempt_bytes INTEGER NOT NULL DEFAULT 0);
         CREATE TABLE IF NOT EXISTS analysis_repair_attempts(
             repair TEXT NOT NULL REFERENCES analysis_repairs(id), attempt INTEGER NOT NULL,
             result TEXT, state TEXT NOT NULL, created INTEGER NOT NULL,
             PRIMARY KEY(repair,attempt));
         CREATE INDEX IF NOT EXISTS analysis_repairs_project ON analysis_repairs(project);",
    )?;
    Ok(())
}

impl Store {
    pub(crate) fn create_analysis_repair(
        &self,
        context: &Context,
        baseline: &Observation,
    ) -> Result<String> {
        let data = bounded_json(baseline)?;
        let digest = baseline.digest()?;
        let mut nonce = [0; 16];
        getrandom::fill(&mut nonce).map_err(|error| failure(error.to_string()))?;
        let id = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                &context.project,
                &context.provider,
                &context.configuration,
                &digest,
                nonce,
            ))?)
        );

        let tx = self.db.unchecked_transaction()?;
        enforce_budget(&tx, &context.project, data.len(), RESERVED_OVERLAY_BYTES)?;
        tx.execute(
            "INSERT INTO analysis_repairs
             (id,project,provider,configuration,baseline,digest,plan,state,created,reserved_attempt_bytes)
             VALUES(?1,?2,?3,?4,?5,?6,NULL,'diagnosing',unixepoch(),?7)",
            params![
                id,
                context.project,
                context.provider,
                context.configuration,
                data,
                digest,
                RESERVED_OVERLAY_BYTES
            ],
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub(crate) fn propose_analysis_repair(
        &self,
        context: &Context,
        id: &str,
        plan: &Plan,
    ) -> Result<()> {
        self.analysis_repair_baseline(context, id)?;
        let data = serde_json::to_string(plan)?;
        if i64::try_from(data.len())? > MAX_PLAN_BYTES {
            return Err(failure("The serialized repair proposal exceeds 64 KiB"));
        }
        let tx = self.db.unchecked_transaction()?;
        enforce_budget(&tx, &context.project, 0, 0)?;
        let changed = tx.execute(
            "UPDATE analysis_repairs SET plan=?2,state='proposed' WHERE id=?1 AND state='diagnosing'",
            params![id, data],
        )?;
        require_change(changed)?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn analysis_repair_baseline(
        &self,
        context: &Context,
        id: &str,
    ) -> Result<Observation> {
        let (project, provider, configuration, data, digest) = self.db.query_row(
            "SELECT project,provider,configuration,baseline,digest FROM analysis_repairs WHERE id=?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;
        if project != context.project
            || provider != context.provider
            || configuration != context.configuration
        {
            return Err(failure(
                "The repair belongs to a different project or analysis configuration",
            ));
        }
        let observation: Observation = serde_json::from_str(&data)?;
        if observation.digest()? != digest {
            return Err(failure("The repair baseline failed its digest check"));
        }
        Ok(observation)
    }

    /// Reserve before provider work so a crash consumes an attempt and leaves an audit record.
    pub(crate) fn begin_analysis_repair_attempt(
        &self,
        context: &Context,
        id: &str,
    ) -> Result<(i64, Plan)> {
        self.analysis_repair_baseline(context, id)?;
        let tx = self.db.unchecked_transaction()?;
        let (state, plan): (String, Option<String>) = tx.query_row(
            "SELECT state,plan FROM analysis_repairs WHERE id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if state == "running" {
            return Err(failure(
                "A repair attempt is already running; recover it after acquiring repair ownership",
            ));
        }
        if !matches!(state.as_str(), "proposed" | "compared") {
            return Err(failure(
                "A repair requires a specialist proposal before rerunning; discarded repairs cannot run",
            ));
        }
        let plan = Plan::parse(&serde_json::from_str::<Value>(
            &plan.ok_or("The repair has no proposal")?,
        )?)?;
        if let Some(reason) = plan.adjustment.blocked_reason() {
            return Err(failure(reason));
        }
        let previous: i64 = tx.query_row(
            "SELECT coalesce(max(attempt),0) FROM analysis_repair_attempts WHERE repair=?1",
            [id],
            |row| row.get(0),
        )?;
        if previous >= MAX_ATTEMPTS {
            return Err(failure(
                "The repair exhausted its two rerun attempts; retain the unresolved result",
            ));
        }
        tx.execute(
            "INSERT INTO analysis_repair_attempts VALUES(?1,?2,NULL,'running',unixepoch())",
            params![id, previous + 1],
        )?;
        tx.execute(
            "UPDATE analysis_repairs SET state='running' WHERE id=?1",
            [id],
        )?;
        tx.commit()?;
        Ok((previous + 1, plan))
    }

    /// Mark abandoned provider work before reserving a new attempt.
    ///
    /// The worker must hold its project ownership lock before calling this method.
    pub(crate) fn interrupt_analysis_repair_attempt(
        &self,
        context: &Context,
        id: &str,
    ) -> Result<()> {
        self.analysis_repair_baseline(context, id)?;
        let tx = self.db.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE analysis_repair_attempts SET state='interrupted',
             result=coalesce(result, '{\"error\":\"provider work was interrupted before capture\"}')
             WHERE repair=?1 AND state='running'",
            [id],
        )?;
        if changed > 0 {
            require_change(tx.execute(
                "UPDATE analysis_repairs SET state='proposed'
                 WHERE id=?1 AND state='running'",
                [id],
            )?)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn finish_analysis_repair_attempt(
        &self,
        context: &Context,
        id: &str,
        attempt: i64,
        result: &Value,
    ) -> Result<()> {
        let baseline = self.analysis_repair_baseline(context, id)?;
        let data = bounded_json(result)?;
        let completion = validate_completion_result(result, &baseline)?;
        let tx = self.db.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE analysis_repair_attempts SET result=?3,state=?4
             WHERE repair=?1 AND attempt=?2 AND state='running'",
            params![id, attempt, data, completion.attempt_state()],
        )?;
        require_change(changed)?;
        require_change(tx.execute(
            "UPDATE analysis_repairs SET state=?2 WHERE id=?1 AND state='running'",
            params![id, completion.repair_state()],
        )?)?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn analysis_repair_status(&self, context: &Context, id: &str) -> Result<Value> {
        let baseline = self.analysis_repair_baseline(context, id)?;
        let (state, plan): (String, Option<String>) = self.db.query_row(
            "SELECT state,plan FROM analysis_repairs WHERE id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let plan: Option<Value> = plan.map(|text| serde_json::from_str(&text)).transpose()?;
        let mut query = self.db.prepare(
            "SELECT attempt,state,result FROM analysis_repair_attempts WHERE repair=?1 ORDER BY attempt",
        )?;
        let rows = query.query_map([id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        let attempts = summarize_attempts(rows, &baseline.digest()?)?;
        Ok(json!({
            "repair": id,
            "state": state,
            "address": baseline.address,
            "signals": crate::analysis_repair::diagnose(&baseline),
            "baseline_digest": baseline.digest()?,
            "plan": plan,
            "attempts": attempts,
            "max_attempts": MAX_ATTEMPTS,
            "provenance": {
                "binary_sha256": &context.project,
                "provider": &context.provider,
                "configuration": &context.configuration
            },
            "authoritative_evidence_changed": false,
            "behavior_verified": false,
            "specialist_reference_verified": false
        }))
    }

    pub(crate) fn analysis_repair_evidence(
        &self,
        context: &Context,
        id: &str,
        attempt: Option<i64>,
    ) -> Result<Value> {
        let baseline = self.analysis_repair_baseline(context, id)?;
        let Some(attempt) = attempt else {
            return Ok(serde_json::to_value(baseline)?);
        };
        let (state, result): (String, Option<String>) = self.db.query_row(
            "SELECT state,result FROM analysis_repair_attempts WHERE repair=?1 AND attempt=?2",
            params![id, attempt],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let result =
            result.ok_or("The attempt has no completed capture; it may have been interrupted")?;
        let result: Value = serde_json::from_str(&result)?;
        if matches!(state.as_str(), "compared" | "incomplete" | "invalidated") {
            validate_attempt_result(&result, &baseline.digest()?)?;
        }
        Ok(result)
    }

    pub(crate) fn discard_analysis_repair(&self, context: &Context, id: &str) -> Result<()> {
        self.analysis_repair_baseline(context, id)?;
        require_change(self.db.execute(
            "UPDATE analysis_repairs SET state='discarded'
             WHERE id=?1 AND state IN ('diagnosing','proposed','compared','incomplete')",
            [id],
        )?)
    }
}

fn bounded_json(value: &impl serde::Serialize) -> Result<String> {
    let data = serde_json::to_string(value)?;
    if data.len() > MAX_CAPTURE_BYTES {
        return Err(failure(
            "The repair capture exceeds 768 KiB; original analysis artifacts were retained",
        ));
    }
    Ok(data)
}

fn require_change(changed: usize) -> Result<()> {
    if changed != 1 {
        return Err(failure(
            "The repair state changed; read its status before continuing",
        ));
    }
    Ok(())
}

fn enforce_budget(
    db: &Connection,
    project: &str,
    additional: usize,
    reservation: i64,
) -> Result<()> {
    let used: i64 = db.query_row(
        "SELECT coalesce(sum(length(CAST(baseline AS BLOB))
             +CASE WHEN reserved_attempt_bytes>0 THEN reserved_attempt_bytes
              ELSE coalesce(length(CAST(plan AS BLOB)),0) END),0)
         + (SELECT coalesce(sum(length(CAST(a.result AS BLOB))),0) FROM analysis_repair_attempts a
            JOIN analysis_repairs r ON r.id=a.repair
            WHERE r.project=?1 AND r.reserved_attempt_bytes=0)
         FROM analysis_repairs WHERE project=?1",
        [project],
        |row| row.get(0),
    )?;
    if used
        .saturating_add(i64::try_from(additional)?)
        .saturating_add(reservation)
        > MAX_PROJECT_BYTES
    {
        return Err(failure(
            "The project's 32 MiB repair budget is full; existing recovery evidence was retained",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum CompletionState {
    Compared,
    Incomplete,
    Invalidated,
}

impl CompletionState {
    fn attempt_state(self) -> &'static str {
        match self {
            Self::Compared => "compared",
            Self::Incomplete => "incomplete",
            Self::Invalidated => "invalidated",
        }
    }

    fn repair_state(self) -> &'static str {
        match self {
            Self::Compared => "compared",
            Self::Incomplete => "proposed",
            Self::Invalidated => "proposed",
        }
    }
}

fn validate_completion_result(result: &Value, baseline: &Observation) -> Result<CompletionState> {
    let candidate = validate_attempt_result(result, &baseline.digest()?)?;
    if candidate.address != baseline.address {
        return Err(failure(
            "The repair candidate does not match the baseline function",
        ));
    }
    let identity_valid = result["comparison"]["capture_identity_valid"]
        .as_bool()
        .ok_or("The repair result must record capture identity validity")?;
    if !identity_valid {
        if result["comparison"]["verdict"].as_str() != Some("invalidated")
            || result["comparison"]["invalidation_reason"]
                .as_str()
                .is_none_or(str::is_empty)
        {
            return Err(failure(
                "An invalid capture must include an invalidation reason",
            ));
        }
        return Ok(CompletionState::Invalidated);
    }

    let complete_capture = result["comparison"]["complete_capture"]
        .as_bool()
        .ok_or("The repair result must record capture completeness")?;
    if complete_capture {
        Ok(CompletionState::Compared)
    } else {
        Ok(CompletionState::Incomplete)
    }
}

fn validate_attempt_result(result: &Value, baseline_digest: &str) -> Result<Observation> {
    let candidate: Observation = serde_json::from_value(
        result
            .get("candidate")
            .ok_or("A repair attempt is missing candidate evidence")?
            .clone(),
    )?;
    let expected = result["comparison"]["candidate_digest"]
        .as_str()
        .ok_or("A repair attempt is missing its candidate digest")?;
    if candidate.digest()? != expected {
        return Err(failure(
            "The stored repair candidate failed its digest check",
        ));
    }
    if result["comparison"]["baseline_digest"].as_str() != Some(baseline_digest) {
        return Err(failure(
            "The stored repair result references a different baseline",
        ));
    }
    Ok(candidate)
}

fn summarize_attempts(
    rows: Vec<(i64, String, Option<String>)>,
    baseline_digest: &str,
) -> Result<Vec<Value>> {
    rows.into_iter()
        .map(|(attempt, state, result)| {
            let result: Option<Value> = result
                .map(|text| -> Result<Value> {
                    let value: Value = serde_json::from_str(&text)?;
                    if matches!(state.as_str(), "compared" | "incomplete" | "invalidated") {
                        validate_attempt_result(&value, baseline_digest)?;
                    }
                    Ok(value)
                })
                .transpose()?;
            Ok(json!({
                "attempt": attempt,
                "state": state,
                "comparison": result.as_ref().map(|value| &value["comparison"])
            }))
        })
        .collect()
}
