use super::Graph;
use crate::analysis::execution::ExecutionTrace;
use serde_json::json;

#[test]
fn relocates_observations_without_inventing_static_edges() -> crate::error::Result<()> {
    let mut graph = Graph::from_dossier(
        &json!({
            "basic_blocks":[
                {"start":"0x1000","end":"0x1010","successors":["0x1010"]},
                {"start":"0x1010","end":"0x1020","successors":[]}
            ],
            "callees":[{"address":"0x1100"}],"callers":[]
        }),
        "",
    )?;
    let trace: ExecutionTrace = serde_json::from_value(json!({
        "version":1,"id":"login-1","workflow":"login","source":"frida_stalker",
        "runtime_version":"fixture","captured_at":"2026-10-26T00:00:00Z",
        "target_sha256":"a".repeat(64),"complete":false,
        "mapping":{"runtime_base":"0x7000","static_base":"0x1000","size":4096},
        "events":[
            {"kind":"call","address":"0x4000","target":"0x7000","count":2,"thread_id":1},
            {"kind":"instruction","address":"0x7010","count":3,"thread_id":1},
            {"kind":"call","address":"0x7014","target":"0x7120","count":4,"thread_id":1},
            {"kind":"call","address":"0x7018","target":"0x9000","count":1,"thread_id":1},
            {"kind":"instruction","address":"0x7020","count":9,"thread_id":1}
        ]
    }))?;
    trace.validate(&"a".repeat(64))?;
    graph.annotate(&[trace]);
    let evidence = &graph.dynamic[0];
    assert_eq!(
        evidence.blocks.len(),
        2,
        "An inbound external call establishes entry execution"
    );
    assert_eq!(evidence.blocks[0].observations, 2);
    assert_eq!(
        evidence.blocks[1].observations, 8,
        "End addresses are exclusive"
    );
    assert_eq!(evidence.calls[0].site, "0x1014");
    assert_eq!(evidence.calls[0].target.as_deref(), Some("0x1120"));
    assert_eq!(
        evidence.calls[1].target, None,
        "External targets must remain unresolved"
    );
    assert_eq!(evidence.unmapped_observations, 1);
    assert_eq!(
        graph.callees,
        ["0x1100"],
        "Observed calls must not overwrite static callees"
    );
    assert_eq!(graph.blocks[0].successors, ["0x1010"]);
    graph.annotate(&[]);
    assert!(
        graph.dynamic.is_empty(),
        "Removing evidence clears annotations on a cached graph"
    );
    Ok(())
}
