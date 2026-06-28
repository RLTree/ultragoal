use serde_json::json;

mod cli_receipts;

#[test]
fn anti_theater_laws_join_to_final_packet_registry_and_cli_authority() {
    let root = crate::self_tests::boundaries::support::temp_root("anti-theater-deps");
    crate::self_tests::audit::final_packet::support::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let law = "generated-proof-artifact-provenance-anti-fabrication";
    let missing = crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures_for_test(
        &root, &store, law,
    );
    for expected in [
        "final_packet_proof_missing",
        "plugin_self_law_json_missing_or_malformed",
        "cli_control_plane_receipt_missing",
    ] {
        assert!(
            missing.iter().any(|failure| failure.contains(expected)),
            "{expected}: {missing:?}"
        );
    }

    let current = crate::package::inventory::package_digest(&root).expect("digest");
    crate::self_tests::audit::final_packet::support::write_green_proof(&root, &current);
    write_cli_pass(
        &root,
        &current,
        "update-goal-eligibility",
        "update_goal_eligibility",
    );
    write_cli_pass(
        &root,
        &current,
        "self-law-receipt",
        "self_update_goal_eligibility",
    );
    let failures =
        crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures_for_test(
            &root, &store, law,
        );
    assert!(failures.is_empty(), "{failures:?}");

    crate::self_tests::audit::final_packet::support::write_fail_closed_proof(&root, &current);
    write_cli_fail_closed(
        &root,
        &current,
        "update-goal-eligibility",
        "update_goal_eligibility",
    );
    write_cli_fail_closed(
        &root,
        &current,
        "self-law-receipt",
        "self_update_goal_eligibility",
    );
    let fail_closed =
        crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures_for_test(
            &root,
            &store,
            "adversarial-packet-tampering-forged-proof-rejection",
        );
    assert!(fail_closed.is_empty(), "{fail_closed:?}");
    std::fs::remove_dir_all(root).expect("cleanup anti-theater deps");
}

fn write_cli_pass(root: &std::path::Path, current: &str, name: &str, operation: &str) {
    let path = root.join(format!("validation_artifacts/cli/{name}.json"));
    crate::self_tests::audit::final_packet::support::write_json(
        &path,
        &json!({
            "schema":"harness-ultragoal.cli-control-plane-receipt.v1",
            "schema_version":"v1",
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane","compatibility_binary":"ultragoal-validator","self_law_state":"self_hosted"},
            "generated_at":"2026-06-27T00:00:00Z",
            "root":".",
            "operation":operation,
            "candidate_digest":current,
            "status":"pass",
            "claim_ceiling":"supports_update_goal_eligibility",
            "blocked_claim_classes":[],
            "required_evidence":[
                "current_red_fixture_report_status_pass",
                "coverage_100_no_uncovered_records",
                "current_cli_performance_pass",
                "current_final_packet_proof_pass",
                "live_registry_or_reviewer_exposure_same_surface_pass",
                "all_89_gates_and_100_stop_conditions_pass"
            ],
            "failure":null,
            "evidence_graph":{
                "candidate_digest": current,
                "evaluation_mode": "production_dereferenced",
                "operation": operation,
                "operation_failures": [],
                "items": evidence_items(current)
            },
            "command_surface":["ultragoal update-goal eligibility"],
            "notes":"test self-hosted control proof"
        }),
    );
}

fn write_cli_fail_closed(root: &std::path::Path, current: &str, name: &str, operation: &str) {
    let path = root.join(format!("validation_artifacts/cli/{name}.json"));
    crate::self_tests::audit::final_packet::support::write_json(
        &path,
        &json!({
            "schema":"harness-ultragoal.cli-control-plane-receipt.v1",
            "schema_version":"v1",
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane","compatibility_binary":"ultragoal-validator","self_law_state":"transition_only"},
            "generated_at":"2026-06-27T00:00:00Z",
            "root":".",
            "operation":operation,
            "candidate_digest":current,
            "status":"fail",
            "claim_ceiling":"withheld_or_blocked",
            "blocked_claim_classes":[
                "completion",
                "package_readiness",
                "review_readiness",
                "release_readiness",
                "update_goal_eligibility"
            ],
            "required_evidence":["current_final_packet_proof_pass"],
            "failure":{"id":format!("{operation}_evidence_not_satisfied"),"law_id":"cli-control-plane-authority"},
            "evidence_graph":{
                "candidate_digest": current,
                "evaluation_mode": "production_dereferenced",
                "operation": operation,
                "operation_failures": ["same-surface proof unavailable"],
                "items": evidence_items(current)
            },
            "command_surface":["ultragoal update-goal eligibility"],
            "notes":"test fail-closed control blocker"
        }),
    );
}

fn evidence_items(current: &str) -> Vec<serde_json::Value> {
    [
        (
            "source_audit",
            "validation_artifacts/ultragoal-audit/validator-receipt.json",
        ),
        (
            "red_fixture_report",
            "validation_artifacts/ultragoal-audit/red-fixture-report.json",
        ),
        (
            "coverage",
            "validation_artifacts/coverage/coverage-receipt.json",
        ),
        (
            "cli_performance",
            "validation_artifacts/cli/performance-receipt.json",
        ),
        (
            "final_packet",
            "validation_artifacts/review/final-packet-proof.json",
        ),
        (
            "registry_exposure",
            "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        ),
        (
            "fit_repo",
            "validation_artifacts/harness/fit-repo-receipt.json",
        ),
        (
            "product_fitness",
            "validation_artifacts/harness/product-fitness-receipt.json",
        ),
        (
            "product_journey",
            "validation_artifacts/harness/plugin-product-journey-receipt.json",
        ),
        (
            "standards_gardener",
            "validation_artifacts/standards-gardener/current-standards-gardening-receipt.json",
        ),
        (
            "rust_toolchain",
            "validation_artifacts/rust/toolchain-receipt.json",
        ),
        ("rust_fast", "validation_artifacts/rust/fast-receipt.json"),
        (
            "rust_standard",
            "validation_artifacts/rust/standard-receipt.json",
        ),
        (
            "rust_release",
            "validation_artifacts/rust/release-receipt.json",
        ),
        (
            "rust_clean_proof",
            "validation_artifacts/rust/clean-proof-receipt.json",
        ),
        ("rust_watch", "validation_artifacts/rust/watch-receipt.json"),
        (
            "rust_memory",
            "validation_artifacts/rust/memory-receipt.json",
        ),
        (
            "rust_dependency",
            "validation_artifacts/rust/dependency-receipt.json",
        ),
        (
            "rust_coverage",
            "validation_artifacts/rust/coverage-receipt.json",
        ),
        (
            "rust_workspace_topology",
            "validation_artifacts/rust/workspace-topology-receipt.json",
        ),
        ("gc_plan", "validation_artifacts/gc/plan-receipt.json"),
        ("gc_dry_run", "validation_artifacts/gc/dry-run-receipt.json"),
        ("gc_apply", "validation_artifacts/gc/apply-receipt.json"),
        ("gc_verify", "validation_artifacts/gc/verify-receipt.json"),
        (
            "transactional_finalization",
            "validation_artifacts/cli/transactional-finalization-receipt.json",
        ),
    ]
    .into_iter()
    .map(|(label, path)| {
        json!({
            "label": label,
            "path": path,
            "exists": true,
            "digest": current,
            "schema": "test-green-receipt.v1",
            "status": "pass",
            "candidate_digest": current,
            "same_candidate": true,
            "failures": []
        })
    })
    .collect()
}
