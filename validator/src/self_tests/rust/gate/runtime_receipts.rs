use crate::cli::garbage::collection::types::GarbageOperation;
use crate::cli::garbage::collection::{
    GarbageCommand, receipt as gc_receipt, run as gc_run, run_with_receipt_value as gc_run_receipt,
};
use crate::cli::rust::cache_with_env;
use crate::cli::rust::types::RustOperation;
use serde_json::json;

#[test]
fn gc_run_without_receipt_and_rust_audit_receipt_ok_path_are_covered() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let gc_code = gc_run(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::Verify,
            receipt: None,
            plan_digest: Some("sha256:test-plan".to_string()),
            apply_receipt_digest: Some(crate::digest::bytes(b"apply")),
        },
    )
    .expect("gc run");
    assert_eq!(gc_code, 0);

    let default_plan = gc_receipt(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::Plan,
            receipt: None,
            plan_digest: None,
            apply_receipt_digest: None,
        },
    )
    .expect("default plan");
    assert!(
        default_plan["deletion_plan"]["plan_digest"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );

    let missing_plan = gc_receipt(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::DryRun,
            receipt: None,
            plan_digest: None,
            apply_receipt_digest: None,
        },
    )
    .expect("missing plan receipt");
    assert_eq!(missing_plan["status"], "fail");
    assert_eq!(
        missing_plan["deletion_plan"]["plan_digest"],
        crate::digest::ZERO
    );
    assert!(
        missing_plan["observation_failures"]
            .as_array()
            .expect("missing plan failures")
            .iter()
            .any(|failure| failure.as_str() == Some("workspace_gc_plan_digest_missing"))
    );

    let parent_file = root.join("target/self-tests/gc-parent-file");
    std::fs::write(&parent_file, "not a directory").expect("gc parent file");
    let write_failure = gc_run_receipt(
        &GarbageCommand {
            operation: GarbageOperation::Plan,
            receipt: Some(parent_file.join("receipt.json")),
            plan_digest: None,
            apply_receipt_digest: None,
        },
        &json!({"status":"pass"}),
    );
    assert!(write_failure.is_err());

    let rust_law = crate::audit::rust::developer::LAWS
        .iter()
        .find(|law| law.id == "rust-command-loop-authority")
        .expect("rust law");
    let mut failures = Vec::new();
    crate::audit::rust::developer::require_receipt(
        &root,
        rust_law,
        "sha256:not-current",
        &mut failures,
    );
    assert!(failures.contains(&"rust_devx_receipt_candidate_digest_mismatch".to_string()));
    assert_eq!(
        cache_with_env(
            RustOperation::CleanProof,
            Some("sccache".to_string()),
            Some("target/isolated".to_string()),
        )["rustc_wrapper"],
        "sccache"
    );
    assert_eq!(
        cache_with_env(RustOperation::Fast, None, None)["cargo_target_dir"],
        "target"
    );
}

#[test]
fn gc_plan_observability_fixtures_cover_red_green_and_tamper_paths() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let green = gc_receipt(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::Plan,
            receipt: None,
            plan_digest: None,
            apply_receipt_digest: None,
        },
    )
    .expect("green gc plan receipt");
    assert_eq!(green["status"], "pass");
    assert_eq!(green["observation_failures"], json!([]));

    let mut red = green.clone();
    red["schema"] = json!("not-workspace-gc");
    let red_failures = crate::cli::garbage::collection::receipt::surface_value_failures(&red);
    assert!(red_failures.contains(&"workspace_gc_receipt_wrong_schema".to_string()));

    let mut tamper = green;
    tamper["observation_failures"] = json!(["raw local spool is not live command telemetry"]);
    let tamper_failures = crate::cli::garbage::collection::receipt::surface_value_failures(&tamper);
    assert!(
        tamper_failures
            .contains(&"workspace_gc_receipt_pass_with_observation_failures".to_string())
    );
}

#[test]
fn gc_apply_observability_fixtures_cover_red_green_and_tamper_paths() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let red = gc_receipt(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::Apply,
            receipt: None,
            plan_digest: None,
            apply_receipt_digest: None,
        },
    )
    .expect("red gc apply receipt");
    assert_eq!(red["status"], "fail");
    assert!(
        red["observation_failures"]
            .as_array()
            .expect("red failures")
            .iter()
            .any(|failure| failure.as_str() == Some("workspace_gc_plan_digest_missing"))
    );

    let green = gc_receipt(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::Apply,
            receipt: None,
            plan_digest: Some(crate::digest::bytes(b"plan")),
            apply_receipt_digest: None,
        },
    )
    .expect("green gc apply receipt");
    assert_eq!(green["status"], "pass");
    assert_eq!(green["observation_failures"], json!([]));

    let mut tamper = red;
    tamper["status"] = json!("pass");
    tamper["claim_ceiling"] = json!("gc_observation_bound");
    let failures = crate::cli::garbage::collection::receipt::surface_value_failures(&tamper);
    assert!(failures.contains(&"workspace_gc_receipt_pass_with_observation_failures".to_string()));
}

#[test]
fn gc_dry_run_observability_fixtures_cover_red_green_and_tamper_paths() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let red = gc_receipt(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::DryRun,
            receipt: None,
            plan_digest: None,
            apply_receipt_digest: None,
        },
    )
    .expect("red gc dry-run receipt");
    assert_eq!(red["status"], "fail");
    assert!(
        red["observation_failures"]
            .as_array()
            .expect("red failures")
            .iter()
            .any(|failure| failure.as_str() == Some("workspace_gc_plan_digest_missing"))
    );

    let green = gc_receipt(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::DryRun,
            receipt: None,
            plan_digest: Some(crate::digest::bytes(b"plan")),
            apply_receipt_digest: None,
        },
    )
    .expect("green gc dry-run receipt");
    assert_eq!(green["status"], "pass");
    assert_eq!(green["observation_failures"], json!([]));

    let mut tamper = red;
    tamper["status"] = json!("pass");
    tamper["claim_ceiling"] = json!("gc_observation_bound");
    let failures = crate::cli::garbage::collection::receipt::surface_value_failures(&tamper);
    assert!(failures.contains(&"workspace_gc_receipt_pass_with_observation_failures".to_string()));
}
