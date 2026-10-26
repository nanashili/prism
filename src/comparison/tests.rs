use super::*;
use crate::model::{Procedure, Project, Target};

fn function(
    address: &str,
    bytes: &str,
    normalized: &str,
    structure: &str,
    callees: &[&str],
) -> Fingerprint {
    Fingerprint {
        function: Procedure {
            address: address.into(),
            name: format!("sub_{address}"),
            kind: "Function".into(),
        },
        bytes: bytes.into(),
        normalized: normalized.into(),
        structure: structure.into(),
        callees: callees.iter().map(|item| (*item).into()).collect(),
        scope: SignatureScope::RecoveredBlocks,
    }
}

fn snapshot(id: &str, functions: Vec<Fingerprint>) -> Snapshot {
    Snapshot {
        project: Project {
            target: Target {
                sha256: id.into(),
                name: id.into(),
                ..Target::default()
            },
            ..Project::default()
        },
        architecture: "Aarch64:Little:MachO".into(),
        functions,
        unknown: Vec::new(),
    }
}

#[test]
fn moved_functions_and_changed_constants_keep_distinct_evidence() -> crate::error::Result<()> {
    let (before, before_mnemonics) = signatures::instructions("0x100: mov w0, #1\n0x104: ret\n")?;
    let (after, after_mnemonics) = signatures::instructions("0x500: mov w0, #2\n0x504: ret\n")?;
    let report = compare(
        snapshot(
            "old",
            vec![
                function("100", "a", &before, &before_mnemonics, &[]),
                function(
                    "200",
                    "stable",
                    "stable-normalized",
                    "stable-structure",
                    &[],
                ),
            ],
        ),
        snapshot(
            "new",
            vec![
                function("500", "b", &after, &after_mnemonics, &[]),
                function(
                    "900",
                    "stable",
                    "stable-normalized",
                    "stable-structure",
                    &[],
                ),
            ],
        ),
    )?;
    assert_eq!(report.matches.len(), 2);
    assert_eq!(report.matches[0].method, Method::NormalizedInstructions);
    assert!(!report.matches[0].bytes_equal);
    assert_eq!(report.matches[1].target.address, "900");
    assert_eq!(report.matches[1].method, Method::ExactBytes);
    Ok(())
}

#[test]
fn duplicate_bodies_remain_ambiguous_until_call_graph_resolves_them() -> crate::error::Result<()> {
    let source = vec![
        function("100", "anchor", "anchor", "anchor", &[]),
        function("200", "dup", "dup", "dup", &["100"]),
        function("300", "dup", "dup", "dup", &[]),
    ];
    let target = vec![
        function("500", "anchor", "anchor", "anchor", &[]),
        function("600", "dup", "dup", "dup", &["500"]),
        function("700", "dup", "dup", "dup", &[]),
    ];
    let report = compare(
        snapshot("old", source.clone()),
        snapshot("new", target.clone()),
    )?;
    assert!(
        report
            .matches
            .iter()
            .any(|item| item.source.address == "200"
                && item.target.address == "600"
                && item.method == Method::CallGraph)
    );
    assert_eq!(report.ambiguous.len(), 1);
    assert_eq!(report.ambiguous[0].source[0].address, "300");
    let report = compare(
        snapshot("old", source[1..].to_vec()),
        snapshot("new", target[1..].to_vec()),
    )?;
    assert!(report.matches.is_empty());
    assert_eq!(report.ambiguous[0].source.len(), 2);
    assert_eq!(report.ambiguous[0].target.len(), 2);
    Ok(())
}

#[test]
fn unknown_extraction_is_not_reported_as_removed() -> crate::error::Result<()> {
    let mut source = snapshot("old", vec![]);
    source.unknown.push(Unknown {
        function: Procedure {
            address: "100".into(),
            ..Procedure::default()
        },
        reason: "Truncated assembly".into(),
    });
    let report = compare(
        source,
        snapshot("new", vec![function("500", "new", "new", "new", &[])]),
    )?;
    assert!(report.source_only.is_empty());
    assert_eq!(report.source_unknown.len(), 1);
    assert_eq!(report.target_only.len(), 1);
    Ok(())
}

#[test]
fn topology_is_address_independent_but_preserves_edges() -> crate::error::Result<()> {
    let original = serde_json::json!({"basic_blocks":[
        {"start":"0x100","end":"0x104","successors":["0x104"]},
        {"start":"0x104","end":"0x108","successors":[]}
    ]});
    let moved = serde_json::json!({"basic_blocks":[
        {"start":"0x500","end":"0x504","successors":["0x504"]},
        {"start":"0x504","end":"0x508","successors":[]}
    ]});
    let changed = serde_json::json!({"basic_blocks":[
        {"start":"0x500","end":"0x504","successors":["0x500"]},
        {"start":"0x504","end":"0x508","successors":[]}
    ]});
    let original = signatures::structure(&signatures::blocks(&original)?, "mnemonics");
    assert_eq!(
        original,
        signatures::structure(&signatures::blocks(&moved)?, "mnemonics")
    );
    assert_ne!(
        original,
        signatures::structure(&signatures::blocks(&changed)?, "mnemonics")
    );
    assert!(signatures::instructions("0x100: ret\nDisplay limited to 1 MiB").is_err());
    Ok(())
}

#[test]
fn transfer_is_atomic_on_conflict_and_saves_provenance() -> crate::error::Result<()> {
    let directory = tempfile::tempdir()?;
    let binary = directory.path().join("binary");
    std::fs::write(&binary, b"synthetic binary")?;
    let target_sha = signatures::digest(b"synthetic binary");
    let mut source = snapshot(
        "old",
        vec![
            function("100", "a", "a", "a", &[]),
            function("200", "b", "b", "b", &[]),
        ],
    );
    source
        .project
        .renamed
        .insert("100".into(), "read_config".into());
    source
        .project
        .notes
        .insert("200".into(), "Confirmed parser; revalidate bounds".into());
    source.project.bookmarks.push("100".into());
    let report = compare(
        source,
        snapshot(
            &target_sha,
            vec![
                function("500", "a", "a", "a", &[]),
                function("600", "b", "b", "b", &[]),
            ],
        ),
    )?;
    let mut destination = Project {
        target: Target {
            sha256: target_sha,
            path: binary,
            ..Target::default()
        },
        ..Project::default()
    };
    destination
        .notes
        .insert("600".into(), "Keep current discovery".into());
    let selections = vec![
        Selection {
            source: "100".into(),
            target: "500".into(),
        },
        Selection {
            source: "200".into(),
            target: "600".into(),
        },
    ];
    assert!(transfer(&report, &destination, &selections).is_err());
    assert!(destination.renamed.is_empty());
    destination.notes.clear();
    let updated = transfer(&report, &destination, &selections)?;
    assert_eq!(updated.renamed["500"], "read_config");
    assert!(updated.notes.contains_key("600"));
    assert_eq!(updated.bookmarks, ["500"]);
    assert_eq!(
        updated.imported["version-transfers.json"][0]["source_sha256"],
        "old"
    );
    let document = directory.path().join("transferred.prism");
    crate::project::save(&document, &updated, &[])?;
    let mut archive = crate::project::archive::Archive::open(&document)?;
    assert_eq!(
        archive.json("version-transfers.json")?,
        updated.imported["version-transfers.json"]
    );
    assert_eq!(
        archive.json("rust-project.json")?["renamed"]["500"],
        "read_config"
    );
    assert!(
        transfer(
            &report,
            &updated,
            &[selections[0].clone(), selections[0].clone()]
        )
        .is_err()
    );
    assert!(
        transfer(
            &report,
            &destination,
            &[Selection {
                source: "100".into(),
                target: "600".into()
            }]
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn native_listing_topology_survives_relocation_and_keeps_scope_explicit() -> crate::error::Result<()>
{
    let before = "0x100: cbz w0, 0x108\n0x104: bl 0x300\n0x108: ret\n";
    let after = "0x500: cbz w0, 0x508\n0x504: bl 0x900\n0x508: ret\n";
    let (_, mnemonics) = signatures::instructions(before)?;
    let original = listing::structure(&listing::parse(before)?, &mnemonics);
    let moved = listing::structure(&listing::parse(after)?, &mnemonics);
    assert_eq!(original.0, moved.0);
    assert_eq!(original.1, ["0x300"]);
    assert_eq!(moved.1, ["0x900"]);
    let changed = after.replace("cbz w0, 0x508", "cbz w0, 0x500");
    assert_ne!(
        original.0,
        listing::structure(&listing::parse(&changed)?, &mnemonics).0
    );
    assert!(listing::parse("0x104: ret\n0x100: nop").is_err());

    let mut destination = function("500", "same", "same", "same", &[]);
    destination.scope = SignatureScope::ListingSpan;
    let report = compare(
        snapshot("old", vec![function("100", "same", "same", "same", &[])]),
        snapshot("new", vec![destination]),
    )?;
    assert!(report.matches.is_empty());
    assert_eq!(report.target_only.len(), 1);
    let mut other_arch = snapshot("new", Vec::new());
    other_arch.architecture = "X86_64:Little:MachO".into();
    assert!(compare(snapshot("old", Vec::new()), other_arch).is_err());
    Ok(())
}
