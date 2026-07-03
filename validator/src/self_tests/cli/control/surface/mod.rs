use crate::cli::control::plane::surface;
use crate::cli::control::plane::types::ControlOperation;
use crate::cli::control::plane::{ControlCommand, run};
use serde_json::json;
use std::path::Path;

mod observability;
mod target;
mod validation;

fn write_json(path: &Path, value: &serde_json::Value) {
    std::fs::create_dir_all(path.parent().expect("json parent")).expect("json parent");
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

pub(super) fn write_package(root: &Path, content: &str, version: &str) {
    std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin dir");
    std::fs::create_dir_all(root.join("docs")).expect("docs dir");
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"harness-ultragoal","version":version}),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[".codex-plugin/plugin.json","docs/a.txt"]}),
    );
    std::fs::write(root.join("docs/a.txt"), content).expect("content");
}

#[test]
fn package_surface_audit_passes_only_for_same_candidate_target() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("surface-source");
    let target = crate::self_tests::boundaries::workspace_fixtures::temp_root("surface-target");
    write_package(&root, "same", "0.0.test");
    write_package(&target, "same", "0.0.test");
    let command = ControlCommand {
        operation: ControlOperation::InstallAudit,
        receipt: None,
        surface_root: Some(target.clone()),
    };
    let value = surface::receipt(&root, &command).expect("surface receipt");
    let digest = crate::package::inventory::package_digest(&root).expect("source digest");

    assert_eq!(value["status"], "pass");
    assert_eq!(value["claim_ceiling"], "surface_package_digest_aligned");
    assert_eq!(value["candidate_digest"], digest);
    assert_eq!(value["target"]["package_digest"], digest);
    assert!(value["target"].get("local_path").is_none());
    assert!(
        surface::same_candidate_pass_failures(&value, &digest, ControlOperation::InstallAudit)
            .is_empty()
    );

    std::fs::write(target.join("docs/a.txt"), "different").expect("mutate target");
    let stale = surface::receipt(&root, &command).expect("stale receipt");
    assert_eq!(stale["status"], "fail");
    assert!(
        stale["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|failure| failure.as_str() == Some("package_surface_digest_mismatch"))
    );
    assert!(
        !surface::same_candidate_pass_or_fail_closed_failures(
            &stale,
            &digest,
            ControlOperation::InstallAudit
        )
        .contains(&"package_surface_audit_target_digest_mismatch".to_string())
    );
    assert!(
        surface::same_candidate_pass_failures(&stale, &digest, ControlOperation::InstallAudit)
            .iter()
            .any(|failure| failure == "package_surface_audit_status_not_pass")
    );

    let mut weak_blocker = stale.clone();
    weak_blocker["blocked_claim_classes"] = json!([]);
    assert!(
        surface::same_candidate_pass_or_fail_closed_failures(
            &weak_blocker,
            &digest,
            ControlOperation::InstallAudit
        )
        .iter()
        .any(|failure| failure
            == "package_surface_audit_fail_closed_missing_blocked_claim:completion")
    );

    std::fs::remove_dir_all(root).expect("cleanup source");
    std::fs::remove_dir_all(target).expect("cleanup target");
}

#[test]
fn package_cache_surface_audit_passes_only_for_same_candidate_target() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("cache-surface-source");
    let target =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("cache-surface-target");
    write_package(&root, "same", "0.0.test");
    write_package(&target, "same", "0.0.test");
    let command = ControlCommand {
        operation: ControlOperation::CacheAudit,
        receipt: None,
        surface_root: Some(target.clone()),
    };
    let value = surface::receipt(&root, &command).expect("cache surface receipt");
    let digest = crate::package::inventory::package_digest(&root).expect("source digest");

    assert_eq!(value["status"], "pass");
    assert_eq!(value["operation"], "cache_audit");
    assert_eq!(value["target"]["surface"], "versioned_cache_package");
    assert_eq!(value["claim_ceiling"], "surface_package_digest_aligned");
    assert_eq!(value["candidate_digest"], digest);
    assert_eq!(value["target"]["package_digest"], digest);
    assert!(value["target"].get("local_path").is_none());
    assert!(
        surface::same_candidate_pass_failures(&value, &digest, ControlOperation::CacheAudit)
            .is_empty()
    );

    std::fs::write(target.join("docs/a.txt"), "different").expect("mutate cache target");
    let stale = surface::receipt(&root, &command).expect("stale cache receipt");
    assert_eq!(stale["status"], "fail");
    assert!(
        stale["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|failure| failure.as_str() == Some("package_surface_digest_mismatch"))
    );
    assert!(
        !surface::same_candidate_pass_or_fail_closed_failures(
            &stale,
            &digest,
            ControlOperation::CacheAudit
        )
        .contains(&"package_surface_audit_target_digest_mismatch".to_string())
    );

    std::fs::remove_dir_all(root).expect("cleanup cache source");
    std::fs::remove_dir_all(target).expect("cleanup cache target");
}

#[test]
fn package_surface_run_requires_receipt_and_reports_missing_target() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("surface-run-print");
    let missing = root.join("missing-target");
    write_package(&root, "same", "0.0.test");
    let without_receipt = run(
        &root,
        &ControlCommand {
            operation: ControlOperation::InstallAudit,
            receipt: None,
            surface_root: Some(missing.clone()),
        },
    )
    .expect_err("surface run requires receipt");
    assert!(without_receipt.contains("missing required argument --receipt"));

    let receipt = std::path::PathBuf::from("validation_artifacts/cli/install-audit-receipt.json");
    let exit = run(
        &root,
        &ControlCommand {
            operation: ControlOperation::InstallAudit,
            receipt: Some(receipt.clone()),
            surface_root: Some(missing),
        },
    )
    .expect("surface run with receipt");
    assert_eq!(exit, 1);
    assert!(root.join(&receipt).exists());
    std::fs::remove_dir_all(root).expect("cleanup surface print");
}

#[test]
fn package_surface_run_reports_receipt_source_and_write_errors() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("surface-run-errors");
    let target =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("surface-write-target");
    write_package(&root, "same", "0.0.test");
    write_package(&target, "same", "0.0.test");
    let receipt_dir =
        std::path::PathBuf::from("validation_artifacts/cli/install-audit-receipt.json");
    std::fs::create_dir_all(root.join(&receipt_dir)).expect("receipt dir");
    let write_error = run(
        &root,
        &ControlCommand {
            operation: ControlOperation::InstallAudit,
            receipt: Some(receipt_dir),
            surface_root: Some(target.clone()),
        },
    )
    .expect_err("directory receipt path must fail");
    assert!(!write_error.is_empty());

    let missing_root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("surface-missing-source");
    let source_error = run(
        &missing_root,
        &ControlCommand {
            operation: ControlOperation::InstallAudit,
            receipt: Some(missing_root.join("validation_artifacts/cli/install-audit-receipt.json")),
            surface_root: Some(target.clone()),
        },
    )
    .expect_err("missing source package must fail");
    assert!(!source_error.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup source");
    std::fs::remove_dir_all(target).expect("cleanup target");
    let _ = std::fs::remove_dir_all(missing_root);
}

#[test]
fn package_surface_receipt_uses_default_root_when_surface_root_is_omitted() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("surface-default-root");
    write_package(&root, "same", "0.0.test");
    let value = surface::receipt(
        &root,
        &ControlCommand {
            operation: ControlOperation::RegistryProbe,
            receipt: None,
            surface_root: None,
        },
    )
    .expect("default-root receipt");
    assert_eq!(value["operation"], "registry_probe");
    assert_eq!(value["target"]["surface"], "unsupported_surface");
    assert!(!surface::supports(ControlOperation::RegistryProbe));
    std::fs::remove_dir_all(root).expect("cleanup default root");
}
