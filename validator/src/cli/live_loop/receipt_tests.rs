use super::{
    LiveLoopAction, LiveLoopCommand,
    context::{PackageTruthSnapshot, package_digest_baseline_ms, verify_cache_hit},
    receipt::{claim_evaluation, failure_class, telemetry_status, where_failed},
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn status_projection_reports_pass_without_failure_fields() {
    let blocker = json!({
        "failure_class": "live_loop_telemetry_reconciliation_missing",
        "where_failed": "loop.measure.fmt_check.telemetry_reconciliation"
    });
    assert_eq!(failure_class("pass", &blocker), "none");
    assert_eq!(where_failed("pass", &blocker), "none");
    assert_eq!(
        failure_class("fail", &blocker),
        "live_loop_telemetry_reconciliation_missing"
    );
    assert_eq!(
        where_failed("fail", &blocker),
        "loop.measure.fmt_check.telemetry_reconciliation"
    );
}

#[test]
fn loop_receipt_claim_evaluation_names_pass_and_blocked_surfaces() {
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: PathBuf::from("validation_artifacts/observability/live-loop-run.json"),
        node_id: None,
        measure_all: false,
    };
    let first_blocker = json!({
        "id": "coverage_prove",
        "why_failed": "coverage receipt is stale"
    });

    let pass = claim_evaluation("pass", &command, &first_blocker);
    assert_eq!(pass["claim_status"], "supported_source_local");
    assert_eq!(
        pass["product_behavior_observed"],
        "ultragoal loop run --tier hot --cache-mode verified-local"
    );
    assert!(
        pass["proof_surface"]
            .as_str()
            .expect("pass proof surface")
            .contains("executed or verified-cache timing proof")
    );
    assert!(
        pass["independent_reconciliation_surface"]
            .as_str()
            .expect("pass reconciliation")
            .contains("logs, metrics, traces")
    );

    let blocked = claim_evaluation("fail", &command, &first_blocker);
    assert_eq!(blocked["claim_status"], "blocked");
    assert!(
        blocked["proof_surface"]
            .as_str()
            .expect("blocked proof surface")
            .contains("no acceleration claim")
    );
    assert_eq!(blocked["first_blocker"]["id"], "coverage_prove");

    let partial = claim_evaluation("partial", &command, &first_blocker);
    assert_eq!(
        partial["claim_status"],
        "withheld_validation_result_available"
    );
    assert!(
        partial["proof_surface"]
            .as_str()
            .expect("partial proof surface")
            .contains("no acceleration claim")
    );
}

#[test]
fn partial_loop_status_maps_to_schema_valid_telemetry_blocked_status() {
    assert_eq!(telemetry_status("pass"), "pass");
    assert_eq!(telemetry_status("fail"), "fail");
    assert_eq!(telemetry_status("partial"), "blocked");
    assert_eq!(telemetry_status("unknown"), "blocked");
}

#[test]
fn cache_hit_verification_fails_stale_or_wrong_digest() {
    assert_eq!(verify_cache_hit("key-a", "key-a"), "pass");
    assert_eq!(
        verify_cache_hit("key-a", "key-b"),
        "fail_stale_or_wrong_digest_cache_hit"
    );
}

#[test]
fn package_digest_baseline_uses_same_candidate_telemetry_only() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-package-baseline");
    let receipt_path = root.join("validation_artifacts/observability/package-digest.json");
    crate::json_boundary::write_json(
        &receipt_path,
        &json!({
            "candidate_digest": "sha256:current",
            "event": {"duration_ms": 123}
        }),
    )
    .expect("package digest telemetry receipt");

    assert_eq!(
        package_digest_baseline_ms(&root, "sha256:current"),
        Some(123)
    );
    assert_eq!(package_digest_baseline_ms(&root, "sha256:stale"), None);
    crate::json_boundary::write_json(
        &receipt_path,
        &json!({
            "candidate_digest": 7,
            "event": {"duration_ms": 123}
        }),
    )
    .expect("malformed package digest telemetry receipt");
    assert_eq!(package_digest_baseline_ms(&root, "sha256:current"), None);
}

#[test]
fn package_truth_snapshot_excludes_builder_contract_resources() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-package-truth");
    std::fs::create_dir_all(root.join("docs/ultragoal-contract-2026-07")).expect("docs");
    std::fs::write(root.join("docs/package.md"), "package").expect("package doc");
    std::fs::write(
        root.join("docs/ultragoal-contract-2026-07/README.md"),
        "builder contract",
    )
    .expect("builder doc");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "resources": [
                "docs/package.md",
                "validation_artifacts/observability/package-digest.json",
                "docs/ultragoal-contract-2026-07/README.md"
            ]
        }),
    )
    .expect("manifest");
    let digest = crate::package::inventory::package_digest(&root)
        .expect_err("builder contract listed as resource is rejected");
    assert!(
        digest.contains("builder contract resource is not a package resource"),
        "{digest}"
    );
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "resources": [
                "docs/package.md",
                "validation_artifacts/observability/package-digest.json"
            ],
            "generated_examples": ["docs/ultragoal-contract-2026-07/README.md"]
        }),
    )
    .expect("manifest");
    let package_digest = crate::digest::bytes(b"package");
    let snapshot = PackageTruthSnapshot::new(&root, &package_digest);
    let summary = snapshot.summary();
    assert_eq!(
        summary["builder_contract_exclusion_status"],
        "excluded_from_package_truth"
    );
    assert_eq!(summary["package_resource_count"], 1);
    assert_eq!(summary["builder_contract_resource_count"], 1);
}

#[test]
fn package_truth_snapshot_fails_closed_after_package_mutation() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-mutation");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/package.md"), "before").expect("package doc");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources": ["docs/package.md"]}),
    )
    .expect("manifest");
    let digest = crate::package::inventory::package_digest(&root).expect("digest");
    let snapshot = PackageTruthSnapshot::new(&root, &digest);
    snapshot
        .verify_current(&root)
        .expect("unchanged package truth");
    std::fs::write(root.join("docs/package.md"), "after").expect("mutate package doc");
    let err = snapshot
        .verify_current(&root)
        .expect_err("mutation after snapshot fails closed");
    assert!(
        err.contains("AuditContext package truth snapshot mutated"),
        "{err}"
    );
    assert!(err.contains("fail_closed_start_new_snapshot"), "{err}");
}
