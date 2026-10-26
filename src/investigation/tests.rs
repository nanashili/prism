use super::{Budget, Graph, Status, TaskState, call, start};
use crate::error::Result;
use crate::model::Procedure;
use crate::storage::Store;
use serde_json::{Value, json};

struct Fixture {
    store: Store,
    folder: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Result<Self> {
        let folder = tempfile::tempdir()?;
        let mut store = Store::open(&folder.path().join("prism.db"))?;
        let mut inventory = store.begin_functions("target")?;
        inventory.append(&[
            Procedure {
                address: "0x100".into(),
                name: "load_project".into(),
                kind: "function".into(),
            },
            Procedure {
                address: "0x200".into(),
                name: "save_project".into(),
                kind: "function".into(),
            },
            Procedure {
                address: "0x300".into(),
                name: "dispatch".into(),
                kind: "function".into(),
            },
        ])?;
        inventory.finish()?;
        Ok(Self { store, folder })
    }

    fn start(&self, budget: Budget) -> Result<Graph> {
        start(
            &self.store,
            "target",
            "Understand how the application loads and saves projects",
            budget,
        )
    }

    fn request(&self, id: &str, action: &str) -> Result<Value> {
        call(
            &self.store,
            "target",
            &json!({"action":action,"investigation":id}),
        )
    }

    fn record(&self, id: &str, next: &Value, report: Value) -> Result<Value> {
        call(
            &self.store,
            "target",
            &json!({
                "action":"complete", "investigation":id,
                "task":next["task"],"attempt":next["attempt"],"report":report,
            }),
        )
    }

    fn claim_synthesis(&self, id: &str) -> Result<Value> {
        for _ in 0..12 {
            let result = self.request(id, "next")?;
            if result["next"]["kind"]["kind"] == "synthesize" {
                return Ok(result["next"].clone());
            }
            self.record(
                id,
                &result["next"],
                json!({"findings":"Supported initial evidence at the requested anchor."}),
            )?;
        }
        Err("Synthesis was never scheduled".into())
    }
}

#[test]
fn evidence_expands_the_graph_and_restart_recovers_the_same_claim() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let graph = fixture.start(Budget::default())?;
    let id = graph.id;
    let survey = fixture.request(&id, "next")?;
    assert_eq!(survey["next"]["kind"]["kind"], "survey");
    fixture.record(
        &id,
        &survey["next"],
        json!({"findings":"Overview entry at 0x100; entry metadata is a lead."}),
    )?;
    let locate = fixture.request(&id, "next")?;
    assert_eq!(locate["next"]["kind"]["kind"], "locate");
    fixture.record(&id, &locate["next"], json!({
        "findings":"Project anchors at 0x100 and 0x200; caller at 0x300 requires investigation.",
        "functions":["0x300","0x300"],
        "unresolved":[{"from":"0x100","to":"0x300","question":"Does the caller validate the project before saving?","evidence":"An indirect call near the validation branch needs argument-flow evidence."}]
    }))?;
    let claimed = fixture.request(&id, "next")?;
    assert_eq!(
        claimed["next"]["kind"]["kind"], "resolve",
        "unresolved relationships take precedence over candidate inspection"
    );
    let usage = claimed["graph"]["usage"].clone();

    fixture.store = Store::open(&fixture.folder.path().join("prism.db"))?;
    let recovered = fixture.request(&id, "next")?;
    assert_eq!(
        recovered["next"], claimed["next"],
        "restart must recover the persisted claim"
    );
    assert_eq!(
        recovered["graph"]["usage"], usage,
        "recovery must not charge another attempt"
    );
    let listed = call(&fixture.store, "target", &json!({"action":"list"}))?;
    assert_eq!(listed["investigations"][0]["id"], id);
    assert!(
        call(
            &fixture.store,
            "another-target",
            &json!({"action":"status","investigation":id})
        )
        .is_err(),
        "a graph belongs to its input hash"
    );

    fixture.record(&id, &recovered["next"], json!({"findings":"Reference and argument evidence establishes the validation path before the save call. Batch job: saved-evidence."}))?;
    let mut inspected = Vec::new();
    for _ in 0..12 {
        let result = fixture.request(&id, "next")?;
        let next = &result["next"];
        if next.is_null() {
            assert_eq!(result["summary"]["status"], "complete");
            break;
        }
        match next["kind"]["kind"].as_str() {
            Some("inspect") => inspected.push(
                next["kind"]["address"]
                    .as_str()
                    .ok_or("missing address")?
                    .to_owned(),
            ),
            Some("synthesize") => assert_eq!(
                inspected.len(),
                3,
                "synthesis waits for all initial and discovered evidence"
            ),
            other => return Err(format!("Unexpected next task: {other:?}").into()),
        }
        fixture.record(&id, next, json!({"findings":"Supported function behavior and failure paths, with addresses and saved batch references."}))?;
    }
    inspected.sort();
    assert_eq!(
        inspected,
        ["0x100", "0x200", "0x300"],
        "repeated leads produce one function task"
    );
    let saved = fixture.store.investigation("target", &id)?;
    assert_eq!(saved.status(), Status::Complete);
    assert!(saved.tasks.iter().all(|task| !task.findings.is_empty()));
    Ok(())
}

#[test]
fn synthesis_discovers_new_work_and_preserves_its_previous_draft() -> Result<()> {
    let fixture = Fixture::new()?;
    let id = fixture.start(Budget::default())?.id;
    let synthesis = fixture.claim_synthesis(&id)?;
    let draft = "The project paths are understood, but the dispatch failure path needs evidence.";
    fixture.record(
        &id,
        &synthesis,
        json!({"findings":draft,"functions":["0x300"]}),
    )?;

    let discovered = fixture.request(&id, "next")?;
    assert_eq!(discovered["next"]["kind"]["address"], "0x300");
    fixture.record(
        &id,
        &discovered["next"],
        json!({"findings":"The dispatch failure returns before any save operation."}),
    )?;
    let resumed = fixture.request(&id, "next")?;
    assert_eq!(resumed["next"]["task"], synthesis["task"]);
    assert_eq!(resumed["next"]["attempt"], 2);
    fixture.record(
        &id,
        &resumed["next"],
        json!({"findings":"The final explanation includes the confirmed dispatch failure path."}),
    )?;

    let saved = fixture.store.investigation("target", &id)?;
    assert_eq!(saved.status(), Status::Complete);
    let synthesis = saved
        .tasks
        .iter()
        .find(|task| task.kind == super::types::Kind::Synthesize)
        .ok_or("Missing synthesis task")?;
    assert_eq!(synthesis.findings.len(), 2);
    assert_eq!(synthesis.findings[0].text, draft);
    assert_eq!(synthesis.findings[1].attempt, 2);
    Ok(())
}

#[test]
fn repeated_synthesis_does_not_reset_the_discovery_depth_budget() -> Result<()> {
    let fixture = Fixture::new()?;
    let id = fixture
        .start(Budget {
            depth: 1,
            ..Budget::default()
        })?
        .id;
    let synthesis = fixture.claim_synthesis(&id)?;
    fixture.record(
        &id,
        &synthesis,
        json!({"findings":"A missing dispatch path needs evidence.","functions":["0x300"]}),
    )?;
    let synthesis = fixture.claim_synthesis(&id)?;
    fixture.record(&id, &synthesis, json!({
        "findings":"Dispatch evidence reveals another unresolved edge.",
        "unresolved":[{"from":"0x300","to":"0x100","question":"Does dispatch retry loading after failure?","evidence":"A reference from dispatch to load requires control-flow evidence."}]
    }))?;

    let limited = fixture.request(&id, "next")?;
    assert!(limited["next"].is_null(), "replanning cannot reset depth");
    assert_eq!(limited["summary"]["status"], "limited");
    call(
        &fixture.store,
        "target",
        &json!({"action":"budget","investigation":id,"budget":Budget::default()}),
    )?;
    assert_eq!(
        fixture.request(&id, "next")?["next"]["kind"]["kind"],
        "resolve"
    );
    Ok(())
}

#[test]
fn invalid_reports_and_database_failures_preserve_the_saved_claim() -> Result<()> {
    let fixture = Fixture::new()?;
    let id = fixture.start(Budget::default())?.id;
    let claim = fixture.request(&id, "next")?;
    let before = fixture.request(&id, "status")?;
    assert!(
        call(
            &fixture.store,
            "target",
            &json!({"action":"pause","investigation":id,"task":1})
        )
        .is_err(),
        "fields from another action must not silently change transition semantics"
    );
    let error = fixture
        .record(
            &id,
            &claim["next"],
            json!({
                "findings":"One known lead and one unindexed lead.", "functions":["0x300","0x999"]
            }),
        )
        .expect_err("an unindexed address must reject the whole report");
    assert!(error.to_string().contains("0x999"), "{error}");
    assert_eq!(fixture.request(&id, "status")?, before);

    let connection = rusqlite::Connection::open(fixture.folder.path().join("prism.db"))?;
    connection.execute_batch("CREATE TRIGGER reject_investigation_update BEFORE UPDATE ON investigations BEGIN SELECT RAISE(ABORT,'simulated write failure'); END;")?;
    let error = fixture.record(&id, &claim["next"], json!({"findings":"Evidence which must not replace the last committed state.","functions":["0x300"]}))
        .expect_err("a failed database write must not acknowledge completion");
    assert!(
        error.to_string().contains("simulated write failure"),
        "{error}"
    );
    assert_eq!(
        fixture.request(&id, "status")?,
        before,
        "both graph expansion and completion remain uncommitted"
    );
    connection.execute_batch("DROP TRIGGER reject_investigation_update;")?;
    fixture.record(
        &id,
        &claim["next"],
        json!({"findings":"The same saved claim can be completed after storage recovers."}),
    )?;
    Ok(())
}

#[test]
fn pause_block_retry_and_stale_updates_preserve_ownership() -> Result<()> {
    let fixture = Fixture::new()?;
    let id = fixture.start(Budget::default())?.id;
    let claim = fixture.request(&id, "next")?;
    fixture.request(&id, "pause")?;
    assert!(
        fixture.request(&id, "next")?["next"].is_null(),
        "pause must stop dispatching, including saved claims"
    );
    call(
        &fixture.store,
        "target",
        &json!({"action":"block","investigation":id,"task":claim["next"]["task"],"attempt":claim["next"]["attempt"],"reason":"The provider is unavailable."}),
    )?;
    assert_eq!(
        fixture.request(&id, "status")?["summary"]["status"],
        "paused"
    );
    fixture.request(&id, "resume")?;
    assert_eq!(
        fixture.request(&id, "next")?["summary"]["status"],
        "blocked"
    );
    call(
        &fixture.store,
        "target",
        &json!({"action":"retry","investigation":id,"task":claim["next"]["task"]}),
    )?;
    let retry = fixture.request(&id, "next")?;
    assert_eq!(retry["next"]["attempt"], 2);
    assert!(
        fixture
            .record(
                &id,
                &claim["next"],
                json!({"findings":"Late result from a stale attempt."})
            )
            .is_err()
    );
    fixture.record(
        &id,
        &retry["next"],
        json!({"findings":"Evidence after the provider recovered."}),
    )?;

    let mut first = fixture.store.investigation("target", &id)?;
    let mut stale = fixture.store.investigation("target", &id)?;
    first.paused = true;
    fixture.store.save_investigation(&mut first)?;
    let revision = stale.revision;
    stale.budget.steps += 1;
    assert!(
        fixture.store.save_investigation(&mut stale).is_err(),
        "a stale snapshot cannot overwrite another connection's update"
    );
    assert_eq!(
        stale.revision, revision,
        "failed writes do not advance local revisions"
    );
    assert!(fixture.store.investigation("target", &id)?.paused);
    Ok(())
}

#[test]
fn step_function_and_depth_limits_keep_unfinished_work_visible() -> Result<()> {
    for resource in ["steps", "functions", "depth"] {
        let fixture = Fixture::new()?;
        let mut budget = Budget::default();
        match resource {
            "steps" => budget.steps = 1,
            "functions" => budget.functions = 1,
            "depth" => budget.depth = 0,
            _ => unreachable!(),
        }
        let id = fixture.start(budget)?.id;
        let mut saw_limit = false;
        for _ in 0..12 {
            let next = fixture.request(&id, "next")?;
            if next["next"].is_null() {
                assert_eq!(next["summary"]["status"], "limited", "resource: {resource}");
                saw_limit = true;
                break;
            }
            fixture.record(&id, &next["next"], json!({"findings":"Supported evidence with an additional indexed lead.","functions":["0x300"]}))?;
        }
        assert!(saw_limit, "resource {resource} must stop dispatch");
        let limited = fixture.store.investigation("target", &id)?;
        assert!(
            limited
                .tasks
                .iter()
                .any(|task| task.state == TaskState::Pending),
            "limits must not silently drop remaining work"
        );
        call(
            &fixture.store,
            "target",
            &json!({"action":"budget","investigation":id,"budget":{"steps":100,"functions":40,"depth":16,"evidence_bytes":1048576}}),
        )?;
        assert!(
            !fixture.request(&id, "next")?["next"].is_null(),
            "explicit budget increase resumes {resource}"
        );
    }
    Ok(())
}

#[test]
fn evidence_limit_rejects_a_report_without_losing_the_running_task() -> Result<()> {
    let fixture = Fixture::new()?;
    let id = fixture
        .start(Budget {
            evidence_bytes: 1024,
            ..Budget::default()
        })?
        .id;
    let claim = fixture.request(&id, "next")?;
    let before = fixture.request(&id, "status")?;
    assert!(
        fixture
            .record(&id, &claim["next"], json!({"findings":"e".repeat(1024)}))
            .is_err()
    );
    assert_eq!(fixture.request(&id, "status")?, before);
    fixture.record(
        &id,
        &claim["next"],
        json!({"findings":"A shorter supported report."}),
    )?;
    Ok(())
}
