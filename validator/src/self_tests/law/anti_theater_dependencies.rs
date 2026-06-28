use serde_json::json;

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
                "items": [
                    {"label":"red_fixture_report","path":"validation_artifacts/ultragoal-audit/red-fixture-report.json","exists":true,"digest":current,"schema":"harness-ultragoal.red-fixture-report.v1","status":"pass","candidate_digest":current,"same_candidate":true,"failures":[]},
                    {"label":"coverage","path":"validation_artifacts/coverage/coverage-receipt.json","exists":true,"digest":current,"schema":"harness-ultragoal.coverage-receipt.v1","status":"pass","candidate_digest":current,"same_candidate":true,"failures":[]},
                    {"label":"cli_performance","path":"validation_artifacts/cli/performance-receipt.json","exists":true,"digest":current,"schema":"harness-ultragoal.cli-performance-receipt.v1","status":"pass","candidate_digest":current,"same_candidate":true,"failures":[]},
                    {"label":"final_packet","path":"validation_artifacts/review/final-packet-proof.json","exists":true,"digest":current,"schema":"harness-ultragoal.final-packet-proof.v1","status":"pass","candidate_digest":current,"same_candidate":true,"failures":[]},
                    {"label":"registry_exposure","path":"validation_artifacts/ultragoal-audit/active-registry-exposure-current.json","exists":true,"digest":current,"schema":"harness-ultragoal.codex-registry-exposure.v1","status":"pass","candidate_digest":current,"same_candidate":true,"failures":[]},
                    {"label":"transactional_finalization","path":"validation_artifacts/cli/transactional-finalization-receipt.json","exists":true,"digest":current,"schema":"harness-ultragoal.cli-transactional-finalization-receipt.v1","status":"pass","candidate_digest":current,"same_candidate":true,"failures":[]}
                ]
            },
            "command_surface":["ultragoal update-goal eligibility"],
            "notes":"test self-hosted control proof"
        }),
    );
}
