use super::super::super::proof;
use super::super::support::{fitted_inventory, write_registry_root, write_valid_fixture};
use serde_json::json;

#[test]
fn observability_registry_rejects_stale_missing_and_mismatched_query_receipts() {
    assert_fitting_failure(
        "observe-proof-missing-receipt",
        |root| {
            std::fs::remove_file(
                root.join("validation_artifacts/observability/fitting/package-digest.json"),
            )
            .expect("remove receipt");
        },
        "observability_command_fitting_receipt_missing:package digest:",
    );

    assert_fitting_failure(
        "observe-proof-receipt-not-current",
        |root| {
            let rel = "validation_artifacts/observability/fitting/package-digest.json";
            let mut receipt = crate::json_boundary::read_json(&root.join(rel)).unwrap();
            receipt["operation"] = json!("wrong.operation");
            crate::json_boundary::write_json(&root.join(rel), &receipt).unwrap();
        },
        "observability_command_fitting_receipt_not_current:package digest:",
    );

    assert_fitting_failure(
        "observe-proof-missing-run",
        |root| {
            let rel = "validation_artifacts/observability/fitting/package-digest.json";
            let mut receipt = crate::json_boundary::read_json(&root.join(rel)).unwrap();
            receipt.as_object_mut().unwrap().remove("run_id");
            crate::json_boundary::write_json(&root.join(rel), &receipt).unwrap();
        },
        "observability_command_fitting_receipt_missing_run_id:package digest:",
    );

    assert_fitting_failure(
        "observe-proof-query-missing",
        |root| {
            std::fs::remove_file(
                root.join("validation_artifacts/observability/fitting/package-digest-logs.json"),
            )
            .expect("remove query");
        },
        "observability_command_fitting_query_missing:package digest:",
    );

    assert_fitting_failure(
        "observe-proof-query-stale",
        |root| {
            let rel = "validation_artifacts/observability/fitting/package-digest-metrics.json";
            let mut query = crate::json_boundary::read_json(&root.join(rel)).unwrap();
            query["candidate_digest"] = json!(crate::digest::ZERO);
            crate::json_boundary::write_json(&root.join(rel), &query).unwrap();
        },
        "observability_command_fitting_query_not_current:package digest:",
    );

    assert_fitting_failure(
        "observe-proof-query-wrong-run",
        |root| {
            let rel = "validation_artifacts/observability/fitting/package-digest-traces.json";
            let mut query = crate::json_boundary::read_json(&root.join(rel)).unwrap();
            query["rows"] = json!([{"candidate_digest": "sha256:wrong", "operation": "wrong"}]);
            crate::json_boundary::write_json(&root.join(rel), &query).unwrap();
        },
        "observability_command_fitting_query_not_same_run:package digest:",
    );
}

#[test]
fn observability_surface_fitted_rows_require_same_surface_receipts() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-surface-proof");
    write_registry_root(&root, fitted_inventory());
    write_valid_fixture(&root);
    let rel = "validation_artifacts/observability/fitting/surface-cli-command-families.json";
    let mut receipt = crate::json_boundary::read_json(&root.join(rel)).unwrap();
    receipt["operation"] = json!("wrong.surface");
    crate::json_boundary::write_json(&root.join(rel), &receipt).unwrap();
    let failures = super::super::fitting_failures(&root);
    assert!(
        failures.iter().any(|item| {
            item.starts_with(
                "observability_surface_fitting_receipt_not_current:cli command families:",
            )
        }),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn observability_command_fitting_accepts_only_fail_closed_command_receipts() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-proof-fail-closed");
    write_registry_root(&root, fitted_inventory());
    write_valid_fixture(&root);
    let rel = "validation_artifacts/observability/fitting/source-audit.json";
    let mut receipt = crate::json_boundary::read_json(&root.join(rel)).unwrap();
    receipt["status"] = json!("fail");
    receipt["claim_ceiling"] = json!("withheld_or_blocked");
    receipt["claim_impact"] =
        json!("source_audit_failed_blocks_readiness_release_completion_update_goal");
    receipt["supported_claims"] = json!([]);
    receipt["blocked_claims"] = json!([
        "completion",
        "readiness",
        "release",
        "final_packet_correctness",
        "reviewer_exposure",
        "update_goal_eligibility"
    ]);
    crate::json_boundary::write_json(&root.join(rel), &receipt).unwrap();
    let failures = super::super::fitting_failures(&root);
    assert!(
        !failures.iter().any(|item| item.contains("source audit")),
        "{failures:?}"
    );
    receipt["claim_impact"] = json!("source audit claim withheld until current repair proof");
    crate::json_boundary::write_json(&root.join(rel), &receipt).unwrap();
    let failures = super::super::fitting_failures(&root);
    assert!(
        !failures.iter().any(|item| item.contains("source audit")),
        "{failures:?}"
    );
    receipt.as_object_mut().unwrap().remove("blocked_claims");
    crate::json_boundary::write_json(&root.join(rel), &receipt).unwrap();
    let failures = super::super::fitting_failures(&root);
    assert!(
        failures.iter().any(|item| {
            item.starts_with("observability_command_fitting_receipt_not_current:source audit:")
        }),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn observability_proof_directly_checks_current_empty_and_malformed_receipt_rows() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-proof-direct");
    let inventory = fitted_inventory();
    write_registry_root(&root, inventory.clone());
    write_valid_fixture(&root);
    let row = inventory["fitting_inventory"]["package digest"]
        .as_object()
        .expect("package digest row");
    let mut failures = Vec::new();
    proof::require_current_receipts(&root, "package digest", row, &mut failures);
    assert!(failures.is_empty(), "{failures:?}");

    let empty = json!({}).as_object().unwrap().clone();
    proof::require_current_receipts(&root, "package digest", &empty, &mut failures);
    assert!(failures.is_empty(), "{failures:?}");

    let rel = "validation_artifacts/observability/fitting/package-digest.json";
    let mut receipt = crate::json_boundary::read_json(&root.join(rel)).unwrap();
    receipt.as_object_mut().unwrap().remove("correlation_id");
    crate::json_boundary::write_json(&root.join(rel), &receipt).unwrap();
    proof::require_current_receipts(&root, "package digest", row, &mut failures);
    assert!(
        failures.iter().any(|failure| {
            failure.starts_with(
                "observability_command_fitting_receipt_missing_correlation_id:package digest",
            )
        }),
        "{failures:?}"
    );

    receipt["correlation_id"] = json!("corr-restored");
    receipt["status"] = json!("pending");
    crate::json_boundary::write_json(&root.join(rel), &receipt).unwrap();
    failures.clear();
    proof::require_current_receipts(&root, "package digest", row, &mut failures);
    assert!(
        failures.iter().any(|failure| {
            failure.starts_with("observability_command_fitting_receipt_not_current:package digest")
        }),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup direct proof");
}

#[test]
fn observability_proof_reports_unavailable_candidates_and_missing_surface_operations() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-proof-no-candidate");
    let row = json!({"receipt_paths":[]});
    let row = row.as_object().unwrap().clone();
    let mut failures = Vec::new();
    proof::require_current_receipts(&root, "package digest", &row, &mut failures);
    assert!(failures.iter().any(|item| {
        item.starts_with(
            "observability_command_fitting_candidate_digest_unavailable:package digest",
        )
    }));

    let root = crate::self_tests::boundaries::support::temp_root("observe-proof-no-operation");
    write_registry_root(&root, fitted_inventory());
    let row = json!({"receipt_paths":[]});
    let row = row.as_object().unwrap().clone();
    failures.clear();
    proof::require_current_surface_receipts(&root, "cli command families", &row, &mut failures);
    assert!(failures.contains(
        &"observability_surface_fitting_operation_missing:cli command families".to_string()
    ));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn observability_inventory_requires_owner_and_next_surface_tracking_together() {
    assert_fitting_failure(
        "observe-proof-owner-without-next",
        |root| {
            let path = root.join("docs/generated/observability/command-inventory.json");
            let mut inventory = crate::json_boundary::read_json(&path).unwrap();
            inventory["fitting_inventory"]["package digest"]
                .as_object_mut()
                .unwrap()
                .remove("next_unfitted_surface");
            crate::json_boundary::write_json(&path, &inventory).unwrap();
        },
        "observability_command_fitting_row_shape_only:package digest",
    );
}

fn assert_fitting_failure(
    label: &str,
    mutate: impl FnOnce(&std::path::Path),
    expected_prefix: &str,
) {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    write_registry_root(&root, fitted_inventory());
    write_valid_fixture(&root);
    mutate(&root);
    let failures = super::super::fitting_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with(expected_prefix)),
        "{expected_prefix}: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
