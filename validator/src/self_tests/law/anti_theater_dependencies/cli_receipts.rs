use serde_json::json;

#[test]
fn anti_theater_rejects_cli_receipt_that_is_neither_pass_nor_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "anti-theater-bad-cli-receipt",
    );
    crate::self_tests::audit::final_packet::receipt_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    crate::self_tests::audit::final_packet::receipt_fixtures::write_fail_closed_proof(
        &root, &current,
    );
    write_bad_cli_receipt(
        &root,
        &current,
        "update-goal-eligibility",
        "update_goal_eligibility",
    );
    write_bad_cli_receipt(
        &root,
        &current,
        "self-law-receipt",
        "self_update_goal_eligibility",
    );

    let failures = crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures(
        &root,
        &store,
        "generated-proof-artifact-provenance-anti-fabrication",
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("cli_control_plane_receipt_not_pass")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup anti-theater bad cli receipt");
}

fn write_bad_cli_receipt(root: &std::path::Path, current: &str, name: &str, operation: &str) {
    crate::self_tests::audit::final_packet::receipt_fixtures::write_json(
        &root.join(format!("validation_artifacts/cli/{name}.json")),
        &json!({
            "schema":"harness-ultragoal.cli-control-plane-receipt.v1",
            "schema_version":"v1",
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane","canonical_binary":"ultragoal","self_law_state":"self_hosted"},
            "generated_at":"2026-06-27T00:00:00Z",
            "root":".",
            "operation":operation,
            "candidate_digest":current,
            "status":"fail",
            "claim_ceiling":"supports_update_goal_eligibility",
            "blocked_claim_classes":[],
            "required_evidence":["current_final_packet_proof_pass"],
            "failure":null,
            "evidence_graph":{
                "candidate_digest": current,
                "evaluation_mode": "production_dereferenced",
                "operation": operation,
                "operation_failures": [],
                "items": super::evidence::items(current)
            },
            "command_surface":["ultragoal update-goal eligibility"],
            "notes":"test malformed control proof"
        }),
    );
}
