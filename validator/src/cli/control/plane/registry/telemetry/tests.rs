use super::*;
use crate::cli::control::plane::{ControlCommand, types::ControlOperation};
use serde_json::json;
use std::time::Instant;

fn root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    root
}

fn command(operation: ControlOperation) -> ControlCommand {
    ControlCommand {
        operation,
        receipt: None,
        surface_root: None,
    }
}

#[test]
fn attach_records_registry_binding_and_claim_blockers() {
    let root = root("control-registry-telemetry-attach");
    let mut value = json!({
        "status": "fail",
        "failure": {
            "observed_value": "registry exposure absent update_goal still blocked",
            "blocked_claim_classes": ["release"]
        },
        "blocked_claim_classes": ["readiness"]
    });
    attach(
        &root,
        &command(ControlOperation::RegistryProbe),
        &mut value,
        Instant::now(),
    )
    .expect("attach");

    assert_eq!(value["status"], "fail");
    assert_eq!(value["failure_class"], "external_live_surface_unavailable");
    assert_eq!(
        value["receipt_observability_binding"]["artifact_path"],
        super::super::ACTIVE_RECEIPT
    );
    assert_eq!(
        value["receipt_observability_binding"]["registry_receipt_path"],
        super::super::ACTIVE_RECEIPT
    );
    let blocked = value["observability"]["blocked_claims"]
        .as_array()
        .expect("blocked claims");
    assert!(blocked.iter().any(|claim| claim == "readiness"));
    assert!(blocked.iter().any(|claim| claim == "release"));
    assert!(
        blocked
            .iter()
            .any(|claim| claim == "update_goal_eligibility")
    );
    assert_eq!(value["task_count"], 3);
    assert_eq!(value["observability"]["event"]["task_count"], 3);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn telemetry_rows_preserve_specific_failure_and_task_counts() {
    assert_eq!(why_failed(&json!({"status":"pass"})), "none");

    let failures = json!({
        "status":"fail",
        "failure":{"observed_failures":["coverage stale", "final_packet stale"]}
    });
    assert_eq!(why_failed(&failures), "coverage stale | final_packet stale");
    assert_eq!(task_count(ControlOperation::CoverageProve, &failures), 2);

    let observed_value = json!({
        "status":"fail",
        "failure":{"observed_value":"coverage stale | registry stale | red_fixture stale"}
    });
    assert_eq!(
        why_failed(&observed_value),
        "coverage stale | registry stale | red_fixture stale"
    );
    assert_eq!(
        task_count(ControlOperation::CoverageProve, &observed_value),
        3
    );

    let failure_id = json!({"status":"fail","failure":{"id":"control-id"}});
    assert_eq!(why_failed(&failure_id), "control-id");
    let fallback = json!({"status":"fail"});
    assert_eq!(why_failed(&fallback), "control_plane_evidence_not_proven");
    assert_eq!(task_count(ControlOperation::CoverageProve, &fallback), 1);
}

#[test]
fn blocked_claims_fail_closed_when_receipt_omits_claim_classes() {
    let fallback = json!({"status":"fail","failure":{"observed_value":"source audit stale"}});
    let blocked = blocked_claims(&fallback);

    assert_eq!(
        blocked,
        vec![
            "readiness".to_string(),
            "release".to_string(),
            "completion".to_string(),
            "update_goal_eligibility".to_string()
        ]
    );

    let malformed = json!({"status":"fail","blocked_claim_classes":"not-an-array"});
    assert_eq!(
        blocked_claims(&malformed),
        vec![
            "readiness".to_string(),
            "release".to_string(),
            "completion".to_string(),
            "update_goal_eligibility".to_string()
        ]
    );
    assert!(blocked_claims(&json!({"status":"pass"})).is_empty());
}

#[test]
fn attach_propagates_observability_spool_write_errors() {
    let root = root("control-registry-telemetry-spool-error");
    std::fs::write(root.join("validation_artifacts"), b"not a directory").expect("block spool dir");
    let mut value = json!({"status":"fail"});

    let error = attach(
        &root,
        &command(ControlOperation::CoverageProve),
        &mut value,
        Instant::now(),
    )
    .expect_err("spool write fails");

    assert!(
        error.contains("validation_artifacts/observability/spool"),
        "{error}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
