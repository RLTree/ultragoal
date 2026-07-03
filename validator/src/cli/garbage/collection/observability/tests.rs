use super::*;
use crate::cli::garbage::collection::GarbageCommand;
use crate::cli::garbage::collection::types::GarbageOperation;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    root
}

fn command(operation: GarbageOperation, receipt: Option<&str>) -> GarbageCommand {
    GarbageCommand {
        operation,
        receipt: receipt.map(PathBuf::from),
        plan_digest: Some(crate::digest::ZERO.to_string()),
        apply_receipt_digest: Some(crate::digest::ZERO.to_string()),
    }
}

#[test]
fn gc_observability_covers_failed_operations_and_default_paths() {
    let root = root("gc-observability-failures");
    let mut dry_run = json!({
        "status": "fail",
        "observation_failures": ["workspace_gc_plan_digest_missing"]
    });
    attach(
        &root,
        &command(GarbageOperation::DryRun, None),
        &mut dry_run,
        Instant::now(),
    )
    .expect("dry-run observability");
    assert_eq!(dry_run["failure_class"], "gc_plan_digest_missing");
    assert_eq!(dry_run["where_failed"], "workspace_gc.command_binding");
    assert_eq!(
        dry_run["receipt_observability_binding"]["artifact_path"],
        "validation_artifacts/gc/dry-run-receipt.json"
    );
    assert_eq!(
        dry_run["receipt_observability_binding"]["command_receipt_path"],
        "validation_artifacts/observability/gc-dry-run.json"
    );

    let mut apply = json!({
        "status": "fail",
        "observation_failures": ["workspace_gc_apply_receipt_digest_missing"]
    });
    attach(
        &root,
        &command(GarbageOperation::Apply, None),
        &mut apply,
        Instant::now(),
    )
    .expect("apply observability");
    assert_eq!(apply["failure_class"], "gc_apply_receipt_digest_missing");
    assert_eq!(
        apply["receipt_observability_binding"]["artifact_path"],
        "validation_artifacts/gc/apply-receipt.json"
    );
    assert_eq!(
        apply["receipt_observability_binding"]["command_receipt_path"],
        "validation_artifacts/observability/gc-apply.json"
    );
}

#[test]
fn gc_observability_propagates_write_errors() {
    let root = root("gc-observability-write-error");
    fs::create_dir_all(root.join("validation_artifacts/observability/gc-plan.json"))
        .expect("blocking directory");
    let mut receipt = json!({"status": "pass", "observation_failures": []});
    let err = attach(
        &root,
        &command(GarbageOperation::Plan, None),
        &mut receipt,
        Instant::now(),
    )
    .expect_err("directory path blocks observability receipt write");
    assert!(err.contains("validation_artifacts/observability/gc-plan.json"));
}

#[test]
fn gc_observability_propagates_spool_write_errors() {
    let root = root("gc-observability-spool-error");
    fs::create_dir_all(root.join("validation_artifacts/observability")).expect("observability dir");
    fs::write(
        root.join("validation_artifacts/observability/spool"),
        "not a directory",
    )
    .expect("blocking spool file");
    let mut receipt = json!({"status": "pass", "observation_failures": []});
    let err = attach(
        &root,
        &command(GarbageOperation::Plan, None),
        &mut receipt,
        Instant::now(),
    )
    .expect_err("file path blocks observability spool write");
    assert!(err.contains("validation_artifacts/observability/spool"));
}

#[test]
fn gc_observability_covers_generic_plan_and_verify_failures() {
    let root = root("gc-observability-plan-verify");
    let mut plan = json!({
        "status": "fail",
        "observation_failures": ["workspace_gc_policy_mismatch"]
    });
    attach(
        &root,
        &command(GarbageOperation::Plan, None),
        &mut plan,
        Instant::now(),
    )
    .expect("plan failure observability");
    assert_eq!(plan["failure_class"], "workspace_gc_command_binding_failed");
    assert_eq!(
        plan["next_repair"],
        "rerun ultragoal gc plan and preserve the emitted plan digest"
    );
    assert_eq!(
        plan["receipt_observability_binding"]["artifact_path"],
        "validation_artifacts/gc/plan-receipt.json"
    );

    let mut verify = json!({
        "status": "fail",
        "observation_failures": ["workspace_gc_apply_receipt_digest_missing"]
    });
    attach(
        &root,
        &command(GarbageOperation::Verify, None),
        &mut verify,
        Instant::now(),
    )
    .expect("verify failure observability");
    assert_eq!(
        verify["next_repair"],
        "rerun ultragoal gc verify with the current gc plan digest and apply receipt digest"
    );
    assert_eq!(
        verify["receipt_observability_binding"]["artifact_path"],
        "validation_artifacts/gc/verify-receipt.json"
    );
}
