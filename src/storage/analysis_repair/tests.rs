use super::*;

fn fixture() -> Result<(Store, Context, Observation)> {
    let db = Connection::open_in_memory()?;
    initialize(&db)?;
    let store = Store {
        db,
        cache: Connection::open_in_memory()?,
    };
    let context = Context {
        project: "binary-a".into(),
        provider: "fixture".into(),
        configuration: "{}".into(),
    };
    let baseline = Observation {
        address: "0x1000".into(),
        pseudocode: Some("original provider output".into()),
        assembly: None,
        dossier: None,
        errors: vec![],
    };
    Ok((store, context, baseline))
}

fn plan(kind: &str) -> Result<Plan> {
    Plan::parse(&json!({
        "specialist":"test-specialist-session",
        "diagnosis":"Transient provider failure; retry the frozen function once",
        "adjustment":{"kind":kind}
    }))
}

fn completed_result(baseline: &Observation, candidate: &Observation) -> Result<Value> {
    Ok(json!({
        "candidate": candidate,
        "comparison": {
            "baseline_digest": baseline.digest()?,
            "candidate_digest": candidate.digest()?,
            "capture_identity_valid": true,
            "complete_capture": true,
            "verdict": "unverified"
        }
    }))
}

#[test]
fn invalidated_capture_is_retained_and_retry_is_bounded() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    store.propose_analysis_repair(&context, &id, &plan("inspect_control_flow")?)?;
    for expected_attempt in 1..=2 {
        let (attempt, _) = store.begin_analysis_repair_attempt(&context, &id)?;
        assert_eq!(attempt, expected_attempt);
        let mut result = completed_result(&baseline, &baseline)?;
        result["comparison"]["capture_identity_valid"] = json!(false);
        result["comparison"]["verdict"] = json!("invalidated");
        result["comparison"]["invalidation_reason"] = json!("binary changed during capture");
        store.finish_analysis_repair_attempt(&context, &id, attempt, &result)?;
        assert_eq!(
            store.analysis_repair_status(&context, &id)?["state"],
            "proposed"
        );
        assert_eq!(
            store.analysis_repair_evidence(&context, &id, Some(attempt))?,
            result
        );
    }
    assert!(store.begin_analysis_repair_attempt(&context, &id).is_err());
    Ok(())
}

#[test]
fn incomplete_capture_is_retained_and_retry_is_permitted() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    store.propose_analysis_repair(&context, &id, &plan("inspect_control_flow")?)?;
    let (attempt, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    let mut result = completed_result(&baseline, &baseline)?;
    result["comparison"]["complete_capture"] = json!(false);

    store.finish_analysis_repair_attempt(&context, &id, attempt, &result)?;

    let status = store.analysis_repair_status(&context, &id)?;
    assert_eq!(status["state"], "proposed");
    assert_eq!(status["attempts"][0]["state"], "incomplete");
    assert_eq!(status["behavior_verified"], false);
    assert_eq!(
        store.analysis_repair_evidence(&context, &id, Some(attempt))?,
        result
    );

    let (retry, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    assert_eq!(retry, 2);
    Ok(())
}

#[test]
fn interrupted_attempt_is_recovered_before_next_reservation() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    store.propose_analysis_repair(&context, &id, &plan("refresh_decompilation")?)?;
    let (first, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    assert_eq!(first, 1);
    assert!(store.begin_analysis_repair_attempt(&context, &id).is_err());

    store.interrupt_analysis_repair_attempt(&context, &id)?;
    let (second, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    let result = completed_result(&baseline, &baseline)?;
    store.finish_analysis_repair_attempt(&context, &id, second, &result)?;

    assert!(store.begin_analysis_repair_attempt(&context, &id).is_err());
    assert_eq!(
        store.analysis_repair_evidence(&context, &id, Some(second))?,
        result
    );
    assert!(
        store
            .analysis_repair_evidence(&context, &id, Some(first))
            .is_ok()
    );
    assert_eq!(
        store.analysis_repair_baseline(&context, &id)?.pseudocode,
        baseline.pseudocode
    );
    Ok(())
}

#[test]
fn discard_rejects_running_attempt_and_preserves_failed_commit() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    store.propose_analysis_repair(&context, &id, &plan("inspect_control_flow")?)?;
    let (attempt, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    assert!(store.discard_analysis_repair(&context, &id).is_err());

    store.db.execute_batch(
        "CREATE TRIGGER fail_comparison BEFORE UPDATE OF state ON analysis_repairs
         WHEN new.state='compared' BEGIN SELECT RAISE(ABORT,'disk write failure fixture'); END;",
    )?;
    let result = completed_result(&baseline, &baseline)?;
    assert!(
        store
            .finish_analysis_repair_attempt(&context, &id, attempt, &result)
            .is_err()
    );
    let status = store.analysis_repair_status(&context, &id)?;
    assert_eq!(status["state"], "running");
    assert_eq!(status["attempts"][0]["state"], "running");
    assert_eq!(
        store.analysis_repair_baseline(&context, &id)?.digest()?,
        baseline.digest()?
    );
    Ok(())
}

#[test]
fn corrupted_completed_candidate_is_rejected() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    store.propose_analysis_repair(&context, &id, &plan("inspect_control_flow")?)?;
    let (attempt, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    let mut result = completed_result(&baseline, &baseline)?;
    result["comparison"]["candidate_digest"] = json!("corrupted");
    assert!(
        store
            .finish_analysis_repair_attempt(&context, &id, attempt, &result)
            .is_err()
    );
    Ok(())
}

#[test]
fn wrong_context_cannot_finish_an_attempt() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    store.propose_analysis_repair(&context, &id, &plan("inspect_control_flow")?)?;
    let (attempt, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    let mut wrong_context = context.clone();
    wrong_context.project = "other-binary".into();
    assert!(
        store
            .finish_analysis_repair_attempt(
                &wrong_context,
                &id,
                attempt,
                &completed_result(&baseline, &baseline)?,
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn successful_discard_retains_evidence_and_blocks_repeat_finish() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    store.propose_analysis_repair(&context, &id, &plan("inspect_control_flow")?)?;
    let (attempt, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    let result = completed_result(&baseline, &baseline)?;
    store.finish_analysis_repair_attempt(&context, &id, attempt, &result)?;
    store.discard_analysis_repair(&context, &id)?;
    assert!(store.discard_analysis_repair(&context, &id).is_err());
    assert_eq!(
        store.analysis_repair_evidence(&context, &id, Some(attempt))?,
        result
    );
    assert!(
        store
            .finish_analysis_repair_attempt(&context, &id, attempt, &result)
            .is_err()
    );
    Ok(())
}

#[test]
fn read_time_corruption_is_rejected() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    store.propose_analysis_repair(&context, &id, &plan("inspect_control_flow")?)?;
    let (attempt, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    let result = completed_result(&baseline, &baseline)?;
    store.finish_analysis_repair_attempt(&context, &id, attempt, &result)?;
    let corrupted = json!({
        "candidate": baseline,
        "comparison": {
            "baseline_digest": "bad",
            "candidate_digest": "bad"
        }
    });
    store.db.execute(
        "UPDATE analysis_repair_attempts SET result=?1 WHERE repair=?2 AND attempt=?3",
        params![serde_json::to_string(&corrupted)?, id, attempt],
    )?;
    assert!(
        store
            .analysis_repair_evidence(&context, &id, Some(attempt))
            .is_err()
    );
    assert!(store.analysis_repair_status(&context, &id).is_err());
    Ok(())
}

#[test]
fn wrong_configuration_and_unsupported_overrides_cannot_execute() -> Result<()> {
    let (store, mut context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    let override_plan = Plan::parse(&json!({
        "specialist":"diagnostic-task",
        "diagnosis":"Calling convention remains uncertain",
        "adjustment":{"kind":"calling_convention","convention":"cdecl"}
    }))?;
    store.propose_analysis_repair(&context, &id, &override_plan)?;
    assert!(store.begin_analysis_repair_attempt(&context, &id).is_err());
    assert_eq!(
        store.analysis_repair_status(&context, &id)?["attempts"],
        json!([])
    );
    context.configuration = "changed-analyzer".into();
    assert!(store.analysis_repair_baseline(&context, &id).is_err());
    Ok(())
}

#[test]
fn reserved_proposal_and_capture_survive_later_quota_pressure() -> Result<()> {
    let (store, context, baseline) = fixture()?;
    let id = store.create_analysis_repair(&context, &baseline)?;
    // Simulate other overlays filling the project quota after this reservation.
    let baseline_bytes = i64::try_from(serde_json::to_vec(&baseline)?.len())?;
    store.db.execute(
        "UPDATE analysis_repairs SET reserved_attempt_bytes=?2 WHERE id=?1",
        params![id, MAX_PROJECT_BYTES - baseline_bytes],
    )?;
    assert!(store.create_analysis_repair(&context, &baseline).is_err());

    store.propose_analysis_repair(&context, &id, &plan("inspect_control_flow")?)?;
    let (attempt, _) = store.begin_analysis_repair_attempt(&context, &id)?;
    let result = completed_result(&baseline, &baseline)?;
    store.finish_analysis_repair_attempt(&context, &id, attempt, &result)?;
    assert_eq!(
        store.analysis_repair_evidence(&context, &id, Some(attempt))?,
        result
    );
    Ok(())
}
