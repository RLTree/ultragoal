use super::*;
use crate::cli::control::plane::{ControlCommand, types::ControlOperation};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn package_surface_telemetry_maps_operation_failure_and_claim_edges() {
    assert_eq!(
        command_name(ControlOperation::InstallAudit),
        "ultragoal install"
    );
    assert_eq!(
        command_name(ControlOperation::RegistryProbe),
        "ultragoal package-surface"
    );
    assert_eq!(
        check_id(ControlOperation::RegistryProbe),
        "package-surface-audit-observability-binding"
    );
    assert_eq!(
        failure_class("package_surface_digest_mismatch"),
        "wrong_digest"
    );
    assert_eq!(
        failure_class("unexpected_surface_state"),
        "package_surface_audit_failed"
    );
    assert_eq!(
        claim_impact(ControlOperation::RegistryProbe, "pass"),
        "supports_package_surface_digest_alignment_only_no_readiness_release_completion_update_goal"
    );
    assert_eq!(
        supported_claims(ControlOperation::RegistryProbe, "pass"),
        vec!["package_surface_digest_alignment".to_string()]
    );
    assert_eq!(
        blocked_claims(&json!({
            "blocked_claim_classes": ["completion", "readiness"],
            "unsupported_claim_classes": ["completion", "release"]
        })),
        vec![
            "completion".to_string(),
            "readiness".to_string(),
            "release".to_string()
        ]
    );
    assert!(blocked_claims(&json!({"blocked_claim_classes": "completion"})).is_empty());
}

#[test]
fn package_surface_telemetry_propagates_spool_write_failures() {
    let root = minimal_root("surface-telemetry-write-failure");
    fs::write(root.join("validation_artifacts"), "not a directory")
        .expect("block observability spool");
    let mut value = json!({
        "status": "fail",
        "failures": ["package_surface_digest_mismatch"],
        "blocked_claim_classes": ["completion"],
        "unsupported_claim_classes": ["release"]
    });
    let receipt_path = PathBuf::from("validation_artifacts/cli/cache-audit-receipt.json");
    let err = attach(
        &root,
        &command_with_receipt(ControlOperation::CacheAudit, &receipt_path),
        &receipt_path,
        &mut value,
        Instant::now(),
    )
    .expect_err("spool write should fail");

    assert!(
        err.contains("validation_artifacts/observability/spool"),
        "{err}"
    );
    fs::remove_dir_all(root).expect("cleanup telemetry write failure");
}

fn command_with_receipt(operation: ControlOperation, receipt: &PathBuf) -> ControlCommand {
    ControlCommand {
        operation,
        receipt: Some(receipt.clone()),
        surface_root: None,
    }
}

fn minimal_root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    root
}
