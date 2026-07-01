use crate::cli::control::plane::types::ControlOperation;
use crate::cli::control::plane::{ControlCommand, run, surface};

#[test]
fn package_surface_run_writes_typed_cache_observability() {
    let root = crate::self_tests::boundaries::support::temp_root("surface-run-source");
    let target = crate::self_tests::boundaries::support::temp_root("surface-run-target");
    super::write_package(&root, "same", "0.0.test");
    super::write_package(&target, "same", "0.0.test");
    let path = root.join("validation_artifacts/cli/cache-audit-receipt.json");
    let exit = run(
        &root,
        &ControlCommand {
            operation: ControlOperation::CacheAudit,
            receipt: Some(path.clone()),
            surface_root: Some(target.clone()),
        },
    )
    .expect("surface run");

    assert_eq!(exit, 0);
    let value = crate::json_boundary::read_json(&path).expect("written receipt");
    assert_eq!(value["schema"], surface::SCHEMA);
    assert_eq!(value["operation"], "cache_audit");
    assert_eq!(value["target"]["surface"], "versioned_cache_package");
    assert_eq!(value["check_id"], "cache-audit-observability-binding");
    assert_eq!(value["observability"]["operation"], "cache_audit");
    assert_eq!(
        value["observability"]["event"]["cache_mode"],
        "package_surface_audit_no_refresh"
    );
    let lines = surface::stdout::lines(&path, &value);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("ultragoal-surface-audit pass"));
    assert!(lines[0].contains("proven=cache_surface_digest_alignment"));
    assert!(lines[0].contains("supported_claims=cache_surface_digest_alignment"));

    std::fs::remove_dir_all(root).expect("cleanup source");
    std::fs::remove_dir_all(target).expect("cleanup target");
}

#[test]
fn package_surface_run_emits_fail_closed_cache_observability() {
    let root = crate::self_tests::boundaries::support::temp_root("surface-cache-fail-source");
    let missing = root.join("missing-cache-target");
    super::write_package(&root, "same", "0.0.test");
    let path = root.join("validation_artifacts/cli/cache-audit-receipt.json");
    let exit = run(
        &root,
        &ControlCommand {
            operation: ControlOperation::CacheAudit,
            receipt: Some(path.clone()),
            surface_root: Some(missing),
        },
    )
    .expect("surface run");

    assert_eq!(exit, 1);
    let value = crate::json_boundary::read_json(&path).expect("written receipt");
    assert_eq!(value["status"], "fail");
    assert_eq!(value["check_id"], "cache-audit-observability-binding");
    assert_eq!(value["why_failed"], "package_surface_target_missing");
    assert_eq!(
        value["claim_impact"],
        "blocks_install_cache_parity_readiness_release_completion_update_goal"
    );
    let lines = surface::stdout::lines(&path, &value);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("ultragoal-surface-audit fail"));
    assert!(lines[0].contains("proven=none"));
    assert!(
        lines[1]
            .contains("failed_law=full-local-observability-stack-integration-non-opaque-failure")
    );
    assert!(lines[1].contains("failed_check=cache-audit-observability-binding"));
    assert!(lines[1].contains("query_logs='ultragoal observe logs query"));
    std::fs::remove_dir_all(root).expect("cleanup source");
}
