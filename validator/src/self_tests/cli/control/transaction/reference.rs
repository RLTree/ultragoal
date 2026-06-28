use crate::cli::control::plane::{proof, types::ControlOperation};
use serde_json::json;

#[test]
fn pass_shaped_transaction_still_dereferences_current_evidence() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-transaction-deref");
    super::write_manifest(&root);
    let expected = crate::package::inventory::package_digest(&root).expect("digest");
    super::write_json(
        &root.join(super::RECEIPT),
        &json!({
            "schema": super::SCHEMA,
            "generated_at": "2026-06-27T00:00:00Z",
            "status": "pass",
            "candidate_digest": expected,
            "claim_ceiling": "supports_update_goal_eligibility",
            "transaction_mode": "same_candidate_atomic_finalization",
            "blocked_claim_classes": [],
            "final_packet": super::missing_ref("validation_artifacts/review/final-packet-proof.json"),
            "registry_exposure": super::missing_ref("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
            "cli_performance": super::missing_ref("validation_artifacts/cli/performance-receipt.json"),
            "coverage": super::missing_ref("validation_artifacts/coverage/coverage-receipt.json")
        }),
    );
    let failures = proof::failures(&root, ControlOperation::UpdateGoalEligibility);
    assert!(
        failures.iter().any(|failure| failure
            .starts_with("cli_control_plane_transaction_ref_digest_mismatch:final_packet")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("registry_exposure"))
    );
    std::fs::remove_dir_all(root).expect("cleanup transaction deref");
}

#[test]
fn matching_reference_digests_still_require_receipt_semantics() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-transaction-semantics");
    super::write_manifest(&root);
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    super::write_receipt_set(&root, &current);
    super::write_transaction(&root, &current);
    let failures = proof::failures(&root, ControlOperation::UpdateGoalEligibility);
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("cli_control_plane_transaction_source_audit")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.ends_with("cli_performance_receipt_update_goal_overclaim")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure == "cli_control_plane_transaction_coverage_not_exact_100"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup transaction semantics");
}

#[test]
fn transaction_rejects_bad_top_level_fields_missing_refs_and_stale_refs() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-transaction-fields");
    super::write_manifest(&root);
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    super::write_receipt_set(&root, &current);
    super::write_json(
        &root.join(super::RECEIPT),
        &json!({
            "schema": "wrong",
            "generated_at": "2026-06-27T00:00:00Z",
            "status": "fail",
            "candidate_digest": crate::self_tests::boundaries::support::sha('d'),
            "claim_ceiling": "withheld_or_blocked",
            "transaction_mode": "manual",
            "blocked_claim_classes": ["completion"],
            "final_packet": {"path":"validation_artifacts/review/not-final.json","digest":crate::digest::ZERO,"status":"fail"},
            "registry_exposure": super::ref_row(&root, "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
            "cli_performance": super::ref_row(&root, "validation_artifacts/cli/performance-receipt.json")
        }),
    );
    let failures = proof::failures(&root, ControlOperation::UpdateGoalEligibility);
    for expected in [
        "cli_control_plane_transaction_wrong_schema",
        "cli_control_plane_transaction_status_not_pass",
        "cli_control_plane_transaction_candidate_digest_mismatch",
        "cli_control_plane_transaction_claim_ceiling_not_update_goal",
        "cli_control_plane_transaction_mode_not_atomic",
        "cli_control_plane_transaction_blocks_claims",
        "cli_control_plane_transaction_ref_wrong_surface:final_packet",
        "cli_control_plane_transaction_ref_missing:coverage",
    ] {
        assert!(
            failures.iter().any(|failure| failure.starts_with(expected)),
            "{expected}: {failures:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup transaction fields");
}

#[cfg(unix)]
#[test]
fn transaction_rejects_symlinked_canonical_reference_surface() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-transaction-symlink");
    super::write_manifest(&root);
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    std::fs::create_dir_all(root.join("actual-review")).expect("actual review");
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("artifact parent");
    std::os::unix::fs::symlink(
        root.join("actual-review"),
        root.join("validation_artifacts/review"),
    )
    .expect("review symlink");
    super::write_receipt_set(&root, &current);
    super::write_transaction(&root, &current);
    let failures = proof::failures(&root, ControlOperation::UpdateGoalEligibility);
    assert!(
        failures.iter().any(|failure| failure
            .starts_with("cli_control_plane_transaction_ref_path_invalid:final_packet")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup transaction symlink");
}

#[test]
fn transaction_rejects_uncovered_coverage_ref_without_source_audit_circularity() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-transaction-stale-refs");
    super::write_manifest(&root);
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    super::write_receipt_set(&root, &current);
    super::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({"schema":"harness-ultragoal.validator-receipt.v1","status":"pass","target_revision":{"kind":"package_digest","value":crate::self_tests::boundaries::support::sha('e')},"checks":{}}),
    );
    super::write_json(
        &root.join("validation_artifacts/coverage/coverage-receipt.json"),
        &json!({"schema":"harness-ultragoal.coverage-receipt.v1","target_revision":{"kind":"package_digest","value":crate::self_tests::boundaries::support::sha('f')},"coverage":{"percent":100.0},"uncovered_records":[{"path":"validator/src/lib.rs"}]}),
    );
    super::write_transaction(&root, &current);
    let mut tx = crate::json_boundary::read_json(&root.join(super::RECEIPT)).expect("tx");
    tx["cli_performance"]["status"] = json!("fail");
    super::write_json(&root.join(super::RECEIPT), &tx);
    let failures = proof::failures(&root, ControlOperation::UpdateGoalEligibility);
    for expected in [
        "cli_control_plane_transaction_ref_status_not_pass:cli_performance",
        "cli_control_plane_transaction_coverage_digest_mismatch",
        "cli_control_plane_transaction_coverage_not_exact_100",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{failures:?}"
        );
    }
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("cli_control_plane_transaction_source_audit")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup transaction stale refs");
}

#[test]
fn transaction_unknown_reference_label_fails_closed() {
    let root = crate::self_tests::boundaries::support::temp_root("transaction-ref-unknown");
    let failures = proof::transaction::unknown_ref_value_failures_for_test(&root);
    assert_eq!(
        failures,
        vec!["cli_control_plane_transaction_ref_unknown:unknown"]
    );
}
