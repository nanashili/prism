use super::*;
use crate::model::Procedure;

const PROJECT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ANALYZER: &str = "fixture-provider:cfg-test";

fn store() -> Result<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    let mut import = store.begin_functions(PROJECT)?;
    import.append(&["0x1000", "0x2000", "0x3000"].map(|address| Procedure {
        address: address.into(),
        name: address.into(),
        kind: "function".into(),
    }))?;
    import.finish()?;
    Ok((directory, store))
}

fn base(function: &str, variable: &str) -> Value {
    json!({"function":function,"variable":variable})
}

fn evidence() -> Value {
    let node = base("0x1000", "arg0");
    let consumer = base("0x2000", "arg0");
    let table = base("0x1000", "dispatch");
    json!({"pointer_size":8,
        "accesses":[
            {"base":node,"offset":0,"kind":"pointer","width":8,"pointee":table,"site":"0x1004 load vptr"},
            {"base":node,"offset":8,"kind":"integer","width":4,"stride":4,"count":3,"site":"0x1010 indexed load; bound=3"},
            {"base":consumer,"offset":12,"kind":"integer","width":4,"site":"0x2004 load second element"},
            {"base":consumer,"offset":24,"kind":"pointer","width":8,"pointee":consumer,"site":"0x2008 load next"},
            {"base":table,"offset":0,"kind":"function_pointer","width":8,"target":"0x1000","site":"0x1020 indirect call"},
            {"base":table,"offset":8,"kind":"function_pointer","width":8,"target":"0x2000","site":"0x1028 indirect call"}
        ],
        "aliases":[
            {"left":node,"right":consumer,"site":"0x1030 passes arg0 unchanged"},
            {"left":consumer,"right":base("0x3000","input"),"site":"0x2030 passes arg0 unchanged"}
        ],
        "names":[{"base":node,"name":"SharedNode"},{"base":table,"name":"DispatchTable"}],
        "prototypes":[
            {"function":"0x2000","convention":"unknown","parameters":[{"kind":"pointer","width":8,"pointee":node}],"site":"0x1030 call"},
            {"function":"0x2000","convention":"system_v64","parameters":[{"kind":"pointer","width":8,"pointee":consumer}],"site":"0x2000 entry"}
        ]
    })
}

fn propose(store: &mut Store, evidence: Value) -> Result<Value> {
    let args = json!({"action":"propose","evidence":evidence});
    let mut external = args.clone();
    external["project"] = PROJECT.into();
    validate_request(&external)?;
    store.recover_types(PROJECT, ANALYZER, &args)
}

fn validate_request(external: &Value) -> Result<()> {
    prism::mcp::validate_call(
        prism::mcp::Mode::Analysis,
        &json!({"name":"recover_types","arguments":external}),
    )?;
    Ok(())
}

fn accept(store: &mut Store, proposal: &Value) -> Result<Value> {
    store.recover_types(
        PROJECT,
        ANALYZER,
        &json!({"action":"accept","proposal":proposal["proposal"]}),
    )
}

#[test]
fn shared_layout_reaches_every_consumer_and_survives_reopen() -> Result<()> {
    let (directory, mut store) = store()?;
    let proposal = propose(&mut store, evidence())?;
    assert_eq!(proposal["consistent"], true);
    assert!(
        store
            .recovered_type_context(PROJECT, ANALYZER, &[])?
            .is_none(),
        "A proposal must not leak into accepted context"
    );
    let checked = store.recover_types(
        PROJECT,
        ANALYZER,
        &json!({"action":"check","proposal":proposal["proposal"]}),
    )?;
    assert_eq!(checked["model"], proposal["model"]);
    accept(&mut store, &proposal)?;

    for function in ["0x1000", "0x2000", "0x3000"] {
        let context = store
            .recovered_type_context(PROJECT, ANALYZER, &[function])?
            .ok_or("Missing types")?;
        let types = context["types"]
            .as_array()
            .ok_or("Missing type definitions")?;
        let node = types
            .iter()
            .find(|item| item["name"] == "SharedNode")
            .ok_or("Missing shared node")?;
        assert_eq!(node["minimum_size"], 32);
        assert_eq!(node["consumers"].as_array().map(Vec::len), Some(3));
        let fields = node["fields"].as_array().ok_or("Missing fields")?;
        assert_eq!(
            fields.len(),
            3,
            "An observed array element must not become an overlapping field"
        );
        assert_eq!(fields[1]["count"], 3);
        assert_eq!(fields[1]["evidence"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            fields[2]["pointee"], node["id"],
            "Recursive references must use the shared type"
        );
        let table = types
            .iter()
            .find(|item| item["name"] == "DispatchTable")
            .ok_or("Missing pointee dependency")?;
        assert_eq!(table["shape"], "possible_vtable");
        assert_eq!(fields[0]["pointee"], table["id"]);
    }
    let before = store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?;
    assert_eq!(before["prototypes"][0]["convention"], "system_v64");
    drop(store);
    let mut reopened = Store::open(&directory.path().join("prism.db"))?;
    assert_eq!(
        reopened.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?,
        before
    );
    assert!(
        reopened
            .recovered_type_context(PROJECT, "another-provider", &[])?
            .is_none()
    );
    assert!(
        reopened
            .recovered_type_context("another-project", ANALYZER, &[])?
            .is_none()
    );
    assert!(
        accept(&mut reopened, &proposal).is_err(),
        "Consumed proposal IDs cannot be replayed"
    );
    Ok(())
}

#[test]
fn conflicting_evidence_cannot_replace_accepted_types() -> Result<()> {
    let (_directory, mut store) = store()?;
    let original = propose(&mut store, evidence())?;
    accept(&mut store, &original)?;
    let before = store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?;
    let cases = [
        json!({"pointer_size":8,"accesses":[{"base":base("0x2000","arg0"),"offset":10,"kind":"float","width":8,"site":"0x2010 conflicting overlay"}]}),
        json!({"pointer_size":8,"prototypes":[{"function":"0x2000","convention":"win64","parameters":[],"site":"0x3004 incompatible call"}]}),
        json!({"pointer_size":8,"accesses":[{"base":base("0x9999","missing"),"offset":0,"kind":"integer","width":4,"site":"unindexed function"}]}),
        json!({"pointer_size":8,"names":[{"base":base("0x3000","unrelated"),"name":"SharedNode"}]}),
    ];
    for case in cases {
        let proposal = propose(&mut store, case)?;
        assert_eq!(
            proposal["consistent"], false,
            "Conflict was not reported: {proposal}"
        );
        assert!(accept(&mut store, &proposal).is_err());
        assert_eq!(
            store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?,
            before,
            "Rejected proposals must preserve every accepted consumer"
        );
    }
    Ok(())
}

#[test]
fn acceptance_rechecks_inventory_and_rolls_back_all_writes() -> Result<()> {
    let (_directory, mut store) = store()?;
    let original = propose(&mut store, evidence())?;
    accept(&mut store, &original)?;
    let before = store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?;
    let proposal = propose(
        &mut store,
        json!({"pointer_size":8,"accesses":[
            {"base":base("0x3000","input"),"offset":32,"kind":"integer","width":4,"site":"0x3004 additional field"}
        ]}),
    )?;
    store.db.execute_batch("CREATE TEMP TRIGGER fail_proposal_delete BEFORE DELETE ON type_proposals BEGIN SELECT RAISE(ABORT,'simulated commit failure'); END;")?;
    assert!(accept(&mut store, &proposal).is_err());
    assert_eq!(
        store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?,
        before,
        "A failure after writing the model must roll back that write"
    );
    store
        .db
        .execute_batch("DROP TRIGGER fail_proposal_delete;")?;

    store.db.execute(
        "DELETE FROM functions WHERE project=?1 AND address='0x3000'",
        [PROJECT],
    )?;
    assert!(
        accept(&mut store, &proposal).is_err(),
        "Acceptance must recheck evidence against the live inventory"
    );
    store.db.execute(
        "INSERT INTO functions VALUES(?1,'0x3000','restored','function')",
        [PROJECT],
    )?;
    let accepted = accept(&mut store, &proposal)?;
    assert_eq!(
        accepted["revision"], 2,
        "Recovery data must survive a failed acceptance"
    );
    assert_eq!(accepted["model"]["conflicts"], json!([]));
    Ok(())
}

#[test]
fn invalid_extents_and_superseded_proposals_are_rejected() -> Result<()> {
    let (_directory, mut store) = store()?;
    let original = propose(&mut store, evidence())?;
    let mut changed = evidence();
    changed["accesses"][1]["count"] = 4.into();
    let replacement = propose(&mut store, changed)?;
    assert!(
        accept(&mut store, &original).is_err(),
        "A superseded proposal must not be accepted"
    );
    accept(&mut store, &replacement)?;
    let before = store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?;
    for (key, value) in [
        ("offset", json!(u64::MAX)),
        ("count", json!(0)),
        ("stride", json!(1)),
        ("width", json!(3)),
    ] {
        let mut invalid = evidence();
        invalid["accesses"][1][key] = value;
        assert!(
            propose(&mut store, invalid).is_err(),
            "Invalid access {key} was accepted"
        );
        assert_eq!(
            store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?,
            before
        );
    }
    Ok(())
}

#[test]
fn corrected_snapshot_replaces_types_for_all_consumers() -> Result<()> {
    let (_directory, mut store) = store()?;
    let original = propose(&mut store, evidence())?;
    accept(&mut store, &original)?;
    let before = store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?;
    let mut observations = before["observations"].clone();
    observations["names"][0]["name"] = "CorrectedNode".into();
    validate_request(&json!({
        "project":PROJECT,"action":"propose","update":"replace","evidence":observations
    }))?;
    let proposal = store.recover_types(
        PROJECT,
        ANALYZER,
        &json!({
            "action":"propose","update":"replace","evidence":observations
        }),
    )?;
    assert_eq!(proposal["consistent"], true);
    assert_eq!(
        store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?,
        before
    );
    accept(&mut store, &proposal)?;
    for function in ["0x1000", "0x2000", "0x3000"] {
        let context = store
            .recovered_type_context(PROJECT, ANALYZER, &[function])?
            .ok_or("Missing corrected context")?;
        let types = context["types"].as_array().ok_or("Missing types")?;
        assert!(types.iter().any(|item| item["name"] == "CorrectedNode"));
        assert!(
            types.iter().all(|item| item["name"] != "SharedNode"),
            "Every consumer must move to the corrected definition"
        );
    }

    let withdrawal = store.recover_types(
        PROJECT,
        ANALYZER,
        &json!({"action":"propose","update":"replace","evidence":{"pointer_size":8}}),
    )?;
    accept(&mut store, &withdrawal)?;
    let withdrawn = store.recover_types(PROJECT, ANALYZER, &json!({"action":"read"}))?;
    assert_eq!(withdrawn["types"], json!([]));
    assert_eq!(withdrawn["prototypes"], json!([]));
    assert_eq!(withdrawn["revision"], 3);
    Ok(())
}

#[test]
fn durable_results_use_the_jobs_analysis_configuration_for_types() -> Result<()> {
    let (_directory, mut store) = store()?;
    let saved = crate::decompilation::Context {
        project: PROJECT.into(),
        provider: "fixture".into(),
        configuration: "{}".into(),
    };
    let analyzer = crate::worker::analysis_cache_identity::namespace(&saved);
    let proposal = store.recover_types(
        PROJECT,
        &analyzer,
        &json!({"action":"propose","evidence":evidence()}),
    )?;
    store.recover_types(
        PROJECT,
        &analyzer,
        &json!({"action":"accept","proposal":proposal["proposal"]}),
    )?;
    let started = crate::decompilation::ExportRunner::start(
        &store,
        &saved,
        &json!({"addresses":["0x2000"]}),
    )?;
    let job = started["job"].as_str().ok_or("Missing durable job")?;
    store.record_decompilation_result(
        job,
        "0x2000",
        "succeeded",
        Some("provider artifact"),
        None,
        1,
    )?;
    let current = crate::decompilation::Context {
        configuration: "new settings".into(),
        ..saved
    };
    let results = crate::decompilation::ExportRunner::results(
        &store,
        job,
        &current,
        &json!({"include_data":true}),
    )?;
    assert_eq!(results["results"][0]["pseudocode"], "provider artifact");
    assert_eq!(results["type_recovery"]["revision"], 1);
    assert_eq!(results["type_recovery"]["provider_types_applied"], false);
    assert!(
        results["type_recovery"]["types"]
            .as_array()
            .is_some_and(|types| types.iter().any(|item| item["name"] == "SharedNode")),
        "Historical artifacts must carry the matching type model"
    );
    Ok(())
}
