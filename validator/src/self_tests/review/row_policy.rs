use serde_json::json;

#[test]
fn review_round_row_policy_rejects_stale_reused_scope_runtime_and_anchor_mismatches() {
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let receipt = json!({
        "review_stage":"material_review",
        "anchor_policy":"full_current_candidate"
    });
    let row = json!({
        "fresh_context": false,
        "reused_reviewer": true,
        "review_scope":"delta",
        "falsification_result":"approved",
        "reviewer_authority":"approval_and_claim_raising",
        "sandbox_mode":"workspace-write",
        "validator_receipt_digest":"wrong",
        "review_target_digest":"wrong",
        "archive_digest":"wrong"
    });
    let mut failures = Vec::new();
    crate::review::round::row::policy::row_policy_errors(
        &row,
        &receipt,
        &anchors,
        "claim-falsifier",
        &mut failures,
    );
    for expected in [
        "review_round_reused_reviewer",
        "review_round_not_full_scope",
        "review_round_wrong_falsification_result",
        "review_round_reviewer_authority_invalid",
        "review_round_sandbox_mismatch",
        "review_round_stale_anchor_digest",
    ] {
        let error_ids = failures
            .iter()
            .map(|failure| failure.error.as_str())
            .collect::<Vec<_>>();
        assert!(
            failures.iter().any(|failure| failure.error == expected),
            "{expected}: {error_ids:?}"
        );
    }
}
