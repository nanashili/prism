use super::model::Concern;
use super::{Observation, compare, diagnose, pseudocode_signals};
use crate::error::Result;
use serde_json::json;

#[test]
fn provider_warnings_request_specialist_review_without_rewriting_output() {
    let code = "/* WARNING: Could not recover jumptable */\n\
                /* WARNING: Unknown calling convention */\n\
                /* WARNING: incompatible type assumptions */\n\
                /* WARNING: Bad instruction at function boundary */\n\
                return unaff_x0;";
    let signals = pseudocode_signals(code);
    let concerns: Vec<_> = signals.iter().map(|signal| &signal.concern).collect();
    assert_eq!(
        concerns,
        [
            &Concern::UnresolvedJumpTable,
            &Concern::CallingConvention,
            &Concern::TypeAssumption,
            &Concern::FunctionBoundary,
            &Concern::SuspiciousOutput,
        ]
    );
    assert!(pseudocode_signals("int add(int a, int b) { return a + b; }").is_empty());
}

fn observation() -> Observation {
    Observation {
        address: "0x1000".into(),
        pseudocode: Some("return 0;".into()),
        assembly: Some(json!("0x1000: ret")),
        errors: Vec::new(),
        dossier: Some(json!({"basic_blocks":[
            {"start":"0x1000","end":"0x1004","successors":[]}
        ]})),
    }
}

#[test]
fn inconsistent_cfg_is_a_lead_and_missing_cfg_is_explicit() {
    let mut observation = observation();
    observation.dossier = Some(json!({"basic_blocks":[
        {"start":"0x1004","end":"0x1010"},
        {"start":"0x1008","end":"0x1014"}
    ]}));
    let signals = diagnose(&observation);
    assert_eq!(signals[0].concern, Concern::FunctionBoundary);

    observation.dossier = Some(json!({"basic_blocks":null}));
    let signals = diagnose(&observation);
    assert_eq!(signals[0].concern, Concern::MissingEvidence);

    // REA 6.7.0 on The Unarchiver returns an empty array with an explicit
    // unavailable CFG facet. It must not imply a malformed function boundary.
    observation.dossier = Some(json!({
        "basic_blocks":[],
        "facet_availability":{"cfg":{"available":false,"reason":"CFG unavailable"}}
    }));
    let signals = diagnose(&observation);
    assert!(
        signals
            .iter()
            .any(|signal| signal.concern == Concern::MissingEvidence)
    );
    assert!(
        !signals
            .iter()
            .any(|signal| signal.concern == Concern::FunctionBoundary)
    );
}

#[test]
fn empty_cfg_and_unresolved_indirect_calls_remain_missing_evidence() -> Result<()> {
    let mut capture = observation();
    capture.dossier = Some(json!({"basic_blocks":[]}));
    let signals = diagnose(&capture);
    assert!(
        signals
            .iter()
            .any(|signal| signal.concern == Concern::MissingEvidence)
    );
    assert!(
        !signals
            .iter()
            .any(|signal| signal.concern == Concern::FunctionBoundary)
    );

    capture = observation();
    let dossier = capture.dossier.as_mut().ok_or("Missing fixture dossier")?;
    dossier["native_api"] = json!({"available":true});
    assert!(diagnose(&capture).is_empty());
    capture.dossier.as_mut().ok_or("Missing fixture dossier")?["limitations"] =
        json!(["Indirect calls and calls through memory remain unresolved"]);
    let signals = diagnose(&capture);
    assert!(
        signals
            .iter()
            .any(|signal| signal.concern == Concern::MissingEvidence)
    );
    Ok(())
}

#[test]
fn structured_native_evidence_detects_conflicts_without_warning_comments() -> Result<()> {
    let mut observation = observation();
    observation
        .dossier
        .as_mut()
        .ok_or("Missing fixture dossier")?["native_api"] = json!({
        "available":true,
        "calling_convention":"unknown",
        "jump_tables":[{"mappings":[{"case_value":null,"target_address":"0x1000"}]}],
        "parameters":[
            {"storage":"x0","data_type":"int"},
            {"storage":"x0","data_type":"char *"}
        ]
    });
    let signals = diagnose(&observation);
    for concern in [
        Concern::CallingConvention,
        Concern::UnresolvedJumpTable,
        Concern::TypeAssumption,
    ] {
        assert!(signals.iter().any(|signal| signal.concern == concern));
    }

    observation
        .dossier
        .as_mut()
        .ok_or("Missing fixture dossier")?["native_api"] = json!({
        "available":true,
        "calling_convention":"__cdecl",
        "jump_tables":[{"mappings":[{"case_value":1,"target_address":"0x1000"}]}],
        "parameters":[{"storage":"x0","data_type":"int"}]
    });
    assert!(diagnose(&observation).is_empty());
    assert!(pseudocode_signals("/* recovered jump table with 3 cases */").is_empty());
    Ok(())
}

#[test]
fn evidence_changes_and_warning_reduction_do_not_verify_a_repair() -> Result<()> {
    let mut baseline = observation();
    baseline.pseudocode = Some("/* WARNING: Unknown calling convention */".into());
    let mut candidate = observation();
    candidate.dossier = Some(json!({"basic_blocks":[
        {"start":"0x1000","end":"0x1008","successors":[]}
    ]}));
    let original_digest = baseline.digest()?;
    let result = compare(&baseline, &candidate)?;
    assert_eq!(result["pseudocode"]["changed"], true);
    assert_eq!(result["assembly"]["changed"], false);
    assert_eq!(result["control_flow"]["changed"], true);
    assert_eq!(
        result["removed_signals"][0]["concern"],
        "calling_convention"
    );
    assert_eq!(result["verdict"], "unverified");
    assert_eq!(result["behavior_verified"], false);
    assert_eq!(baseline.digest()?, original_digest);

    candidate.pseudocode = None;
    candidate.errors.push("provider timeout".into());
    let failed = compare(&baseline, &candidate)?;
    assert!(failed["pseudocode"]["changed"].is_null());
    assert_eq!(failed["complete_capture"], false);
    assert_eq!(failed["verdict"], "unverified");

    candidate.dossier = Some(json!({
        "basic_blocks":[],
        "facet_availability":{"cfg":{"available":false}}
    }));
    let unavailable = compare(&baseline, &candidate)?;
    assert!(unavailable["control_flow"]["changed"].is_null());
    assert!(unavailable["control_flow"]["candidate_digest"].is_null());
    Ok(())
}
