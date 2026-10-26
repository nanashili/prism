use super::*;
use crate::analysis::semantic::Feature;
use crate::model::Procedure;

const PROJECT: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SCOPE: &str = "test-provider-configuration";

fn populate(store: &mut Store, documents: &[Document]) -> Result<()> {
    let mut inventory = store.begin_functions(PROJECT)?;
    let functions: Vec<_> = documents
        .iter()
        .map(|document| Procedure {
            address: document.address.clone(),
            name: document.name.clone(),
            kind: "function".into(),
        })
        .collect();
    inventory.append(&functions)?;
    inventory.finish()?;
    store.index_search_documents(SCOPE, documents, &AtomicBool::new(false))
}

fn search(store: &Store, scope: &str, query: &str) -> Result<Vec<Hit>> {
    store.search_functions(&SearchRequest {
        scope,
        project: PROJECT,
        query: &Query::parse(query)?,
        limit: 20,
        similar_to: None,
        cancelled: &AtomicBool::new(false),
    })
}

#[test]
fn behavior_search_ranks_body_evidence_above_a_misleading_name_and_survives_restart() -> Result<()>
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("prism.db");
    let mut store = Store::open(&path)?;
    populate(
        &mut store,
        &[
            Document {
                address: "0x100".into(),
                name: "FUN_100".into(),
                decompiler:
                    "fread(buffer, 16, 1, stream); if (memcmp(buffer, magic, 4)) return -1;".into(),
                imports: vec!["fread".into(), "memcmp".into()],
                constants: vec!["0x464c457f".into()],
                ..Document::default()
            },
            Document {
                address: "0x200".into(),
                name: "FUN_200".into(),
                decompiler: "item = free_list; free_list = item->next; return item;".into(),
                control_flow: vec!["conditional branch".into()],
                ..Document::default()
            },
            Document {
                address: "0x300".into(),
                name: "parse_binary_header_object_pool".into(),
                decompiler:
                    "int parse_binary_header_object_pool(void) { puts(message); return 0; }".into(),
                ..Document::default()
            },
        ],
    )?;
    drop(store);
    let store = Store::open(&path)?;

    for (query, expected) in [
        ("Find functions that parse a binary header", "0x100"),
        ("Find code that implements an object pool", "0x200"),
    ] {
        let hits = search(&store, SCOPE, query)?;
        assert_eq!(
            hits.first().map(|hit| hit.address.as_str()),
            Some(expected),
            "query {query:?}"
        );
        let hit = &hits[0];
        let misleading = hits.iter().find(|hit| hit.address == "0x300");
        assert!(misleading.is_none_or(|hit| {
            hit.evidence
                .iter()
                .all(|evidence| evidence.source == "name")
        }));
        assert!(
            hit.evidence
                .iter()
                .any(|evidence| evidence.source == "inferred_behavior"),
            "inferred behavior should be identified for {query:?}"
        );
        assert!(
            hit.evidence
                .iter()
                .all(|evidence| !evidence.excerpt.is_empty()),
            "each reason must be inspectable"
        );
    }
    Ok(())
}

fn signature(address: &str, summary: &str, weights: &[f64]) -> Document {
    Document {
        address: address.into(),
        name: format!("FUN_{}", address),
        summary: summary.into(),
        structure_space: Some("weights-v1".into()),
        structure: weights
            .iter()
            .zip(1..)
            .map(|(weight, hash)| Feature {
                hash,
                weight: *weight,
            })
            .collect(),
        ..Document::default()
    }
}

#[test]
fn structure_expands_a_behavior_lead_to_a_stripped_function_with_normalized_cosine() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    let mut incompatible = signature("0x400", "", &[1.0, 1.0, 1.0]);
    incompatible.structure_space = Some("different-weights".into());
    populate(
        &mut store,
        &[
            signature("0x100", "deserialize packet", &[1.0, 1.0, 1.0]),
            signature("0x200", "", &[1.0, 1.0, 2.0]),
            signature("0x300", "", &[1.0, 1.0, 10.0]),
            incompatible,
        ],
    )?;

    let hits = search(&store, SCOPE, "parse packet")?;
    assert_eq!(hits.first().map(|hit| hit.address.as_str()), Some("0x100"));
    let twin = hits
        .iter()
        .find(|hit| hit.address == "0x200")
        .ok_or("stripped twin was not retrieved")?;
    let structural = twin
        .structural
        .as_ref()
        .ok_or("missing native structural explanation")?;
    assert_eq!(structural.exemplar, "0x100");
    assert!(
        (structural.cosine - 4.0 / 18.0_f64.sqrt()).abs() < 1e-10,
        "weighted cosine must normalize both vectors"
    );
    assert_eq!(structural.shared_features, 3);
    assert!(
        twin.evidence.is_empty(),
        "the stripped match must not acquire invented textual evidence"
    );
    assert!(
        !hits.iter().any(|hit| hit.address == "0x400"),
        "incompatible native structural configurations must never be compared"
    );
    Ok(())
}

#[test]
fn native_features_retrieve_renamed_bodies_without_inventing_behavior_evidence() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    let bodies = [
        (
            "0x100",
            "object pool",
            "void *take(void) { void *x = free_list; if(x) free_list = x->next; return x; }",
        ),
        (
            "0x200",
            "",
            "void *f(void) { /* irrelevant comment */ void *y = global_2; if(y) global_2 = y->field_0; return y; }",
        ),
        ("0x300", "", "int g(void) { return 7; }"),
    ];
    let documents: Vec<_> = bodies
        .into_iter()
        .map(|(address, summary, code)| Document {
            address: address.into(),
            name: "unknown".into(),
            summary: summary.into(),
            decompiler: code.into(),
            structure_space: Some(crate::analysis::semantic::structure::SPACE.into()),
            structure: crate::analysis::semantic::structure::features(code, None),
            ..Document::default()
        })
        .collect();
    populate(&mut store, &documents)?;

    let hits = search(&store, SCOPE, "object pool")?;
    let twin = hits
        .iter()
        .find(|hit| hit.address == "0x200")
        .ok_or("native twin missing")?;
    let structural = twin.structural.as_ref().ok_or("missing structural route")?;
    assert!(structural.cosine > 0.999);
    assert!(
        twin.evidence.is_empty(),
        "a similar body is not direct evidence of the query"
    );
    assert!(!hits.iter().any(|hit| hit.address == "0x300"));
    Ok(())
}

#[test]
fn failed_refresh_preserves_previous_text_and_structural_index() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    populate(
        &mut store,
        &[
            signature("0x100", "parse packet", &[1.0, 1.0, 1.0]),
            signature("0x200", "", &[1.0, 1.0, 1.0]),
        ],
    )?;
    let replacement = signature("0x100", "encrypt secret", &[1.0, 2.0, 3.0]);
    let invalid = signature("0x200", "", &[1.0, 0.0, 1.0]);
    assert!(
        store
            .index_search_documents(SCOPE, &[replacement, invalid], &AtomicBool::new(false))
            .is_err()
    );
    assert!(search(&store, SCOPE, "encrypt secret")?.is_empty());
    assert!(
        search(&store, SCOPE, "parse packet")?
            .iter()
            .any(|hit| hit.address == "0x200" && hit.structural.is_some())
    );
    assert!(
        store
            .index_search_documents(
                SCOPE,
                &[signature("0x100", "encrypt secret", &[])],
                &AtomicBool::new(true)
            )
            .is_err()
    );
    assert!(search(&store, SCOPE, "encrypt secret")?.is_empty());
    Ok(())
}

#[test]
fn search_scopes_inventory_and_handles_literal_query_syntax() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    populate(
        &mut store,
        &[signature("0x100", "decrypt network packet", &[])],
    )?;
    assert!(search(&store, "another-configuration", "decrypt network")?.is_empty());
    let hits = search(&store, SCOPE, "\"decrypt\" OR (network):*")?;
    assert_eq!(hits.first().map(|hit| hit.address.as_str()), Some("0x100"));
    assert!(Query::parse("find functions that").is_err());

    store.begin_functions(PROJECT)?.finish()?;
    assert!(search(&store, SCOPE, "decrypt network")?.is_empty());
    let coverage = store.function_search_coverage(SCOPE, PROJECT)?;
    assert_eq!(coverage.indexed_functions, 0);
    assert_eq!(coverage.inventory_functions, 0);
    Ok(())
}

#[test]
fn resetting_a_binary_removes_old_configurations_and_preserves_other_indexes() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    populate(
        &mut store,
        &[signature("0x100", "parse packet", &[1.0, 1.0, 1.0])],
    )?;
    let scope = format!("function-search-v1:{PROJECT}:old-runtime");
    store.index_search_documents(
        &scope,
        &[signature("0x100", "decrypt message", &[2.0, 1.0, 1.0])],
        &AtomicBool::new(false),
    )?;
    assert_eq!(
        store.clear_function_search(PROJECT, &AtomicBool::new(false))?,
        1
    );
    assert!(search(&store, &scope, "decrypt message")?.is_empty());
    assert!(!search(&store, SCOPE, "parse packet")?.is_empty());
    assert!(store.search_document(&scope, "0x100")?.is_none());
    Ok(())
}
