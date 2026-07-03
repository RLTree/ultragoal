use super::super::checks::roundtrip_path;
use super::super::{receipt, run, run_command_roundtrip, run_with_timeout};
use super::{command, roundtrip_root};
use crate::audit::observability::specs;
use serde_json::json;
use std::time::Instant;

#[test]
fn command_roundtrip_refuses_to_fit_without_same_candidate_query_roundtrip() {
    let (root, candidate) = roundtrip_root("observe-roundtrip-partial");
    let spec = specs::command("package digest").expect("package spec");

    let row = run_command_roundtrip(&root, spec, 1).expect("roundtrip row");
    assert_eq!(row["command_id"], "package digest");
    assert_eq!(row["roundtrip_status"], "partial");
    assert_eq!(
        row["receipt_path"],
        "validation_artifacts/observability/package-digest.json"
    );
    assert_eq!(row["stdout_receipt_same_candidate"], true);
    assert_eq!(row["explain_status"], "pass");
    assert_eq!(
        crate::json_boundary::read_json(&root.join(roundtrip_path(spec, "explain-failure")))
            .expect("explain")["candidate_digest"],
        candidate
    );
    std::fs::remove_dir_all(root).expect("cleanup roundtrip partial");
}

#[test]
fn observe_fit_run_fails_closed_for_unknown_and_partial_targets() {
    let (root, _) = roundtrip_root("observe-roundtrip-run");
    let mut unknown = command();
    unknown.target_command = Some("install audit".to_string());
    assert!(
        run(&root, &unknown)
            .expect_err("unknown target")
            .contains("unknown observability command spec")
    );

    let mut family = command();
    family.target_command = None;
    family.target_family = Some("package".to_string());
    family.timeout_ms = 1;
    let receipt = run_with_timeout(&root, &family, 1).expect("partial family receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["roundtrip_status"], "partial");
    assert_eq!(receipt["target_family"], "package");
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|claim| claim == "observability_product_closure")
    );
    std::fs::remove_dir_all(root).expect("cleanup roundtrip run");
}

#[test]
fn observe_fit_receipt_keeps_source_local_claim_ceiling() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-roundtrip-receipt");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let row = json!({"roundtrip_status": "observable"});
    let value = receipt(
        &root,
        &command(),
        "sha256:fit".to_string(),
        vec![row],
        Instant::now(),
    )
    .expect("roundtrip receipt");
    assert_eq!(value["status"], "pass");
    assert_eq!(value["roundtrip_status"], "observable");
    assert_eq!(
        value["supported_claims"][0],
        "spec_driven_observability_command_roundtrip_increment"
    );
    assert!(
        value["blocked_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|claim| claim == "update_goal_eligibility")
    );
    let partial = receipt(
        &root,
        &command(),
        "sha256:fit".to_string(),
        vec![json!({"roundtrip_status": "partial"})],
        Instant::now(),
    )
    .expect("partial roundtrip receipt");
    assert_eq!(partial["status"], "fail");
    assert_eq!(
        partial["why_failed"],
        "observability command roundtrip is incomplete"
    );
}
