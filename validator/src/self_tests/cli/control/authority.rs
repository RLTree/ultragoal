use crate::cli::control::plane::types::ControlOperation;
use crate::cli::control::plane::{ControlCommand, receipt};
use serde_json::json;
use std::path::Path;

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn control_plane_receipt_failure_is_evidence_derived() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-control-authority");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let command = ControlCommand {
        operation: ControlOperation::UpdateGoalEligibility,
        receipt: None,
        surface_root: None,
    };

    let value = receipt(&root, &command).expect("receipt");

    assert_eq!(value["status"], "fail");
    assert_eq!(
        value["failure"]["id"],
        "update_goal_eligibility_evidence_not_satisfied"
    );
    assert_ne!(
        value["failure"]["observed_value"],
        "transition_only_or_incomplete_self_law"
    );
    assert!(
        value["failure"]["observed_value"]
            .as_str()
            .expect("observed")
            .contains("schema_catalog")
    );
    assert!(
        value["notes"]
            .as_str()
            .expect("notes")
            .contains("Evidence failures")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn production_control_plane_does_not_use_source_audit_as_green_path() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-no-source-audit-loop");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let command = ControlCommand {
        operation: ControlOperation::UpdateGoalEligibility,
        receipt: None,
        surface_root: None,
    };

    let value = receipt(&root, &command).expect("receipt");
    let observed = value["failure"]["observed_value"]
        .as_str()
        .expect("observed");

    assert_eq!(value["status"], "fail");
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");
    assert!(!observed.contains("source_audit_not_pass"), "{observed}");
    assert!(
        !observed.contains("source_audit_target_digest_mismatch"),
        "{observed}"
    );
    assert!(
        observed.contains("final_packet") || observed.contains("red-fixture-report"),
        "{observed}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
