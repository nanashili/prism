use super::{ClaimDraft, EvidenceSource, SourceKind};
use crate::error::Result;
use crate::storage::{DecompilationJobSpec, Store};
use serde_json::{Value, json};

fn capture(
    store: &Store,
    project: &str,
    source: &str,
    kind: SourceKind,
    data: Value,
) -> Result<String> {
    let receipt = store.capture_evidence(
        project,
        &EvidenceSource {
            key: source,
            subject: "0x14001a20",
            kind,
        },
        &data,
    )?;
    Ok(receipt["id"]
        .as_str()
        .ok_or("Capture omitted its identifier")?
        .into())
}

fn draft(evidence: &str, pointer: &str, predicate: Value) -> Result<ClaimDraft> {
    Ok(serde_json::from_value(json!({
        "statement":"The captured comparison uses the expected checksum",
        "scope":"predicate",
        "checks":[{"evidence":evidence,"pointer":pointer,"predicate":predicate}]
    }))?)
}

fn propose(store: &Store, project: &str, draft: &ClaimDraft) -> Result<i64> {
    store.propose_claim(project, draft)?["id"]
        .as_i64()
        .ok_or_else(|| "Claim omitted its identifier".into())
}

#[test]
fn source_changes_reopen_claims_and_revisions_survive_restart() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("prism.db");
    let store = Store::open(&path)?;
    let original = capture(
        &store,
        "binary",
        "comparison",
        SourceKind::StaticAnalysis,
        json!({"equal":true}),
    )?;
    let check = draft(
        &original,
        "/equal",
        json!({"operation":"equals","expected":true}),
    )?;
    let id = propose(&store, "binary", &check)?;
    assert_eq!(
        store.verification_claim("binary", id)?["assessment"]["status"],
        "proposed"
    );
    assert_eq!(
        store.verify_claim("binary", id)?["assessment"]["status"],
        "corroborated"
    );

    let changed = capture(
        &store,
        "binary",
        "comparison",
        SourceKind::StaticAnalysis,
        json!({"equal":false}),
    )?;
    assert_eq!(
        store.verification_claim("binary", id)?["assessment"]["status"],
        "unresolved"
    );
    assert_eq!(
        store.verification_evidence("binary", &original)?["data"]["equal"],
        true
    );
    assert_eq!(
        store.verification_evidence("binary", &original)?["current"],
        false
    );
    // Re-verification never silently substitutes a newer snapshot for the citation.
    assert_eq!(
        store.verify_claim("binary", id)?["assessment"]["confidence"],
        "none"
    );
    let revised = draft(
        &changed,
        "/equal",
        json!({"operation":"equals","expected":true}),
    )?;
    store.revise_claim("binary", id, &revised)?;
    assert_eq!(
        store.verify_claim("binary", id)?["assessment"]["status"],
        "contradicted"
    );
    drop(store);

    let store = Store::open(&path)?;
    let claim = store.verification_claim("binary", id)?;
    assert_eq!(claim["assessment"]["status"], "contradicted");
    let history = claim["history"].as_array().ok_or("Missing claim history")?;
    assert!(
        history
            .iter()
            .any(|entry| entry["assessment"]["status"] == "corroborated")
    );
    assert!(
        history
            .iter()
            .any(|entry| entry["draft"]["checks"][0]["evidence"] == original)
    );
    assert!(store.verification_claim("another-binary", id).is_err());
    assert!(
        store
            .verification_evidence("another-binary", &original)
            .is_err()
    );
    assert!(store.revise_claim("another-binary", id, &revised).is_err());
    Ok(())
}

#[test]
fn assessment_distinguishes_observations_contradictions_and_incomplete_evidence() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(&directory.path().join("prism.db"))?;
    let cases = [
        (
            SourceKind::StaticAnalysis,
            "/equal",
            json!({"operation":"equals","expected":true}),
            "predicate",
            "corroborated",
            "direct",
        ),
        (
            SourceKind::StaticAnalysis,
            "/equal",
            json!({"operation":"equals","expected":false}),
            "predicate",
            "contradicted",
            "direct",
        ),
        (
            SourceKind::StaticAnalysis,
            "/missing",
            json!({"operation":"equals","expected":null}),
            "predicate",
            "unresolved",
            "none",
        ),
        (
            SourceKind::StaticAnalysis,
            "/code",
            json!({"operation":"contains","text":"memcmp"}),
            "predicate",
            "corroborated",
            "limited",
        ),
        (
            SourceKind::StaticAnalysis,
            "/code",
            json!({"operation":"contains","text":"crc32"}),
            "predicate",
            "unresolved",
            "none",
        ),
        (
            SourceKind::Runtime,
            "/reply/equal",
            json!({"operation":"equals","expected":true}),
            "predicate",
            "runtime-observed",
            "observed",
        ),
        (
            SourceKind::Runtime,
            "/reply/code",
            json!({"operation":"contains","text":"memcmp"}),
            "predicate",
            "corroborated",
            "limited",
        ),
        (
            SourceKind::Runtime,
            "/arguments/equal",
            json!({"operation":"equals","expected":true}),
            "predicate",
            "unresolved",
            "none",
        ),
        (
            SourceKind::StaticAnalysis,
            "/code",
            json!({"operation":"contains","text":"memcmp"}),
            "interpretation",
            "unresolved",
            "none",
        ),
    ];
    for (index, (kind, pointer, predicate, scope, status, confidence)) in
        cases.into_iter().enumerate()
    {
        let evidence = capture(
            &store,
            "binary",
            &format!("source-{index}"),
            kind,
            json!({
                "equal":true,
                "code":"return memcmp(actual, expected, 4) == 0;",
                "reply":{"equal":true,"code":"return memcmp(actual, expected, 4) == 0;"},
                "arguments":{"equal":true}
            }),
        )?;
        let mut value = serde_json::to_value(draft(&evidence, pointer, predicate)?)?;
        value["scope"] = scope.into();
        let draft = serde_json::from_value(value)?;
        let id = propose(&store, "binary", &draft)?;
        let result = store.verify_claim("binary", id)?;
        assert_eq!(result["assessment"]["status"], status, "case {index}");
        assert_eq!(
            result["assessment"]["confidence"], confidence,
            "case {index}"
        );
    }
    Ok(())
}

#[test]
fn failed_capture_preserves_snapshots_but_reopens_claims() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(&directory.path().join("prism.db"))?;
    let original = capture(
        &store,
        "binary",
        "source",
        SourceKind::StaticAnalysis,
        json!({"equal":true}),
    )?;
    let id = propose(
        &store,
        "binary",
        &draft(
            &original,
            "/equal",
            json!({"operation":"equals","expected":true}),
        )?,
    )?;
    store.verify_claim("binary", id)?;
    store.db.execute_batch(
        "CREATE TEMP TRIGGER fail_capture BEFORE INSERT ON verification_evidence
         BEGIN SELECT RAISE(ABORT,'simulated disk failure'); END;",
    )?;
    let error = capture(
        &store,
        "binary",
        "source",
        SourceKind::StaticAnalysis,
        json!({"equal":false}),
    )
    .expect_err("the injected write failure must propagate");
    assert!(error.to_string().contains("simulated disk failure"));
    assert_eq!(
        store.verification_evidence("binary", &original)?["data"]["equal"],
        true
    );
    assert_eq!(
        store.verification_claim("binary", id)?["assessment"]["status"],
        "unresolved"
    );
    store.db.execute_batch("DROP TRIGGER fail_capture")?;
    assert!(
        store
            .propose_claim(
                "binary",
                &draft(
                    &original,
                    "/equal",
                    json!({"operation":"equals","expected":true})
                )?
            )
            .is_err()
    );
    // Identical content observed again produces a new citation, never revives the old one.
    let fresh = capture(
        &store,
        "binary",
        "source",
        SourceKind::StaticAnalysis,
        json!({"equal":true}),
    )?;
    assert_ne!(fresh, original);
    assert_eq!(
        store.verification_evidence("binary", &original)?["current"],
        false
    );
    Ok(())
}

#[test]
fn configuration_and_runtime_changes_invalidate_only_their_dependencies() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(&directory.path().join("prism.db"))?;
    store.verification_context("binary", "provider-v1")?;
    let static_id = capture(
        &store,
        "binary",
        "static",
        SourceKind::StaticAnalysis,
        json!(true),
    )?;
    let runtime_id = capture(
        &store,
        "binary",
        "runtime",
        SourceKind::Runtime,
        json!({"reply":true}),
    )?;
    let other = capture(
        &store,
        "other",
        "static",
        SourceKind::StaticAnalysis,
        json!(true),
    )?;
    let predicate = json!({"operation":"equals","expected":true});
    let static_claim = propose(&store, "binary", &draft(&static_id, "", predicate.clone())?)?;
    let runtime_claim = propose(&store, "binary", &draft(&runtime_id, "/reply", predicate)?)?;
    store.verify_claim("binary", static_claim)?;
    assert_eq!(
        store.verify_claim("binary", runtime_claim)?["assessment"]["status"],
        "runtime-observed"
    );
    store.invalidate_runtime_evidence("binary")?;
    assert_eq!(
        store.verification_claim("binary", runtime_claim)?["assessment"]["status"],
        "unresolved"
    );
    assert_eq!(
        store.verification_claim("binary", static_claim)?["assessment"]["status"],
        "corroborated"
    );
    store.verification_context("binary", "provider-v1")?;
    assert_eq!(
        store.verification_evidence("binary", &static_id)?["current"],
        true
    );
    store.verification_context("binary", "provider-v2")?;
    assert_eq!(
        store.verification_claim("binary", static_claim)?["assessment"]["status"],
        "unresolved"
    );
    assert_eq!(
        store.verification_evidence("other", &other)?["current"],
        true
    );
    Ok(())
}

#[test]
fn new_durable_pseudocode_reopens_claims_without_waiting_for_an_agent_read() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    let mut functions = store.begin_functions("binary")?;
    functions.append(&[crate::model::Procedure {
        address: "0x14001a20".into(),
        name: "validate".into(),
        kind: "function".into(),
    }])?;
    functions.finish()?;
    let digest = store.decompilation_inventory_digest("binary", "", None)?;
    store.create_decompilation_job_with_items(DecompilationJobSpec {
        id: "job",
        project: "binary",
        provider: "provider",
        selection: "{}",
        options: "{}",
        expected_inventory_digest: &digest,
        filter: "",
        addresses: None,
    })?;
    let evidence = capture(
        &store,
        "binary",
        "view:Pseudocode:0x14001a20",
        SourceKind::StaticAnalysis,
        json!({"text":"return validate_checksum(input);"}),
    )?;
    let id = propose(
        &store,
        "binary",
        &draft(
            &evidence,
            "/text",
            json!({"operation":"contains","text":"validate_checksum"}),
        )?,
    )?;
    store.verify_claim("binary", id)?;
    assert!(store.record_decompilation_result(
        "job",
        "0x14001a20",
        "succeeded",
        Some("return 0;"),
        None,
        1
    )?);
    assert_eq!(
        store.verification_claim("binary", id)?["assessment"]["status"],
        "unresolved"
    );
    assert_eq!(
        store.verification_evidence("binary", &evidence)?["current"],
        false
    );
    Ok(())
}

#[test]
fn invalid_citations_and_confidence_cannot_enter_a_claim() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(&directory.path().join("prism.db"))?;
    let unknown = "0".repeat(64);
    assert!(
        store
            .propose_claim(
                "binary",
                &draft(&unknown, "", json!({"operation":"equals","expected":true}))?
            )
            .is_err()
    );
    let mut no_evidence = draft(&unknown, "", json!({"operation":"equals","expected":true}))?;
    no_evidence.checks.clear();
    assert!(store.propose_claim("binary", &no_evidence).is_err());
    let mut caller_confidence = serde_json::to_value(no_evidence)?;
    caller_confidence["confidence"] = json!(1.0);
    assert!(serde_json::from_value::<ClaimDraft>(caller_confidence).is_err());
    assert_eq!(store.verification_claims("binary", 0)?["claims"], json!([]));
    Ok(())
}

#[test]
fn corrupt_evidence_removes_corroboration_and_preserves_the_failure_reason() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(&directory.path().join("prism.db"))?;
    let evidence = capture(
        &store,
        "binary",
        "source",
        SourceKind::StaticAnalysis,
        json!(true),
    )?;
    let id = propose(
        &store,
        "binary",
        &draft(&evidence, "", json!({"operation":"equals","expected":true}))?,
    )?;
    store.verify_claim("binary", id)?;
    store.db.execute(
        "UPDATE verification_evidence SET data='false' WHERE id=?1",
        [&evidence],
    )?;
    let result = store.verify_claim("binary", id)?;
    assert_eq!(result["assessment"]["status"], "unresolved");
    assert!(
        result["assessment"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("digest check"))
    );
    assert!(store.verification_evidence("binary", &evidence).is_err());
    // Deduplication must not issue a fresh receipt for a corrupt stored snapshot.
    assert!(
        capture(
            &store,
            "binary",
            "source",
            SourceKind::StaticAnalysis,
            json!(true)
        )
        .is_err()
    );
    let fresh = capture(
        &store,
        "binary",
        "source",
        SourceKind::StaticAnalysis,
        json!(true),
    )?;
    assert_ne!(fresh, evidence);
    assert_eq!(store.verification_evidence("binary", &fresh)?["data"], true);
    Ok(())
}
