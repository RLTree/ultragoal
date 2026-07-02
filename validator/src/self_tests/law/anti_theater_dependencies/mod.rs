use serde_json::json;

mod cli_receipts;
mod evidence;

#[test]
fn anti_theater_laws_join_to_final_packet_registry_and_cli_authority() {
    let root = crate::self_tests::boundaries::support::temp_root("anti-theater-deps");
    crate::self_tests::audit::final_packet::support::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let law = "generated-proof-artifact-provenance-anti-fabrication";
    let missing = crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures(
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
    let failures = crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures(
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
    let fail_closed = crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures(
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
                crate::cli::control::plane::emit::FULL_CONTRACT_EVIDENCE
            ],
            "failure":null,
            "evidence_graph":{
                "candidate_digest": current,
                "evaluation_mode": "production_dereferenced",
                "operation": operation,
                "operation_failures": [],
                "items": evidence::items(current)
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
                "final_packet_correctness",
                "update_goal_eligibility"
            ],
            "required_evidence":["current_final_packet_proof_pass"],
            "failure":{"id":format!("{operation}_evidence_not_satisfied"),"law_id":"cli-control-plane-authority"},
            "evidence_graph":{
                "candidate_digest": current,
                "evaluation_mode": "production_dereferenced",
                "operation": operation,
                "operation_failures": ["same-surface proof unavailable"],
                "items": evidence::items(current)
            },
            "command_surface":["ultragoal update-goal eligibility"],
            "notes":"test fail-closed control blocker"
        }),
    );
}
