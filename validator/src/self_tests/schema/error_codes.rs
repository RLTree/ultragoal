use serde_json::json;

#[test]
fn schema_error_codes_cover_authority_specific_failure_classes() {
    let cases = [
        (
            vec!["required_claim_ids must be non-empty".to_string()],
            "required_claim_missing",
        ),
        (
            vec!["plugin_manifest.skills missing required ultragoal".to_string()],
            "required_skill_missing",
        ),
        (
            vec!["verification_backlog.rows[0].attempts is required".to_string()],
            "blocker_without_attempt_evidence",
        ),
        (
            vec!["required_check_ids contains duplicate values".to_string()],
            "duplicate_validator_check_rows",
        ),
        (
            vec!["required_red_fixture_ids contains duplicate values".to_string()],
            "duplicate_red_fixture_rows",
        ),
        (
            vec!["validator_receipt.commit must match sha256".to_string()],
            "validator_commit_value_not_sha",
        ),
        (
            vec!["validator_receipt.target_revision.value must match sha256".to_string()],
            "package_digest_value_not_sha",
        ),
        (
            vec!["commands[0].exit const mismatch".to_string()],
            "command_receipt_missing_or_failed",
        ),
        (
            vec!["worktree_clean const mismatch".to_string()],
            "dirty_self_reported_clean",
        ),
        (
            vec!["teardown_ready const mismatch".to_string()],
            "stale_worktree_after_closeout",
        ),
        (
            vec!["blocked_reasons violates maxItems".to_string()],
            "ready_receipt_has_blockers",
        ),
        (
            vec!["actor_binding.validated_at format date-time".to_string()],
            "actor_validation_timestamp_malformed",
        ),
        (
            vec!["last_heartbeat format date-time".to_string()],
            "lane_heartbeat_timestamp_malformed",
        ),
        (
            vec!["actor_status const mismatch".to_string()],
            "stale_actor_identity",
        ),
        (
            vec!["root_verification_stages.final_all_lanes_gate is required".to_string()],
            "root_verification_stage_missing_or_duplicate",
        ),
        (
            vec!["unrecognized validation failure".to_string()],
            "schema_validation_failed",
        ),
    ];
    for (errors, expected) in cases {
        assert_eq!(
            crate::schema_catalog::schema_error_code(&errors),
            expected,
            "{errors:?}"
        );
    }
}

#[test]
fn schema_dispatch_reaches_product_cohesion_rules() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let store = crate::schema_catalog::load(&root);
    let errors = crate::schema_catalog::schema_errors(
        &store,
        "product-cohesion-receipt.schema.json",
        &json!({}),
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("product_surface_id is required")),
        "{errors:?}"
    );
}
