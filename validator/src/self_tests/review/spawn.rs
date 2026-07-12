use serde_json::json;

fn has_fail(failures: &[crate::review::round::ReviewFailure], error: &str) -> bool {
    failures.iter().any(|failure| failure.error == error)
}

#[test]
fn review_round_spawn_receipts_fail_missing_failed_unknown_and_stale_paths() {
    let receipt = json!({"round_id":"round-1","generated_at":"2026-06-26T09:00:00Z"});
    let mut failures = Vec::new();
    crate::review::round::spawn::receipts::spawn_receipt_errors(
        &receipt,
        &json!({}),
        "claim-falsifier",
        &mut failures,
    );
    assert!(has_fail(&failures, "review_round_live_spawn_missing"));

    failures.clear();
    crate::review::round::spawn::receipts::spawn_receipt_errors(
        &receipt,
        &json!({
            "reviewer_agent_id":"agent-1",
            "runtime_metadata":{"exposure":"unavailable"},
            "sandbox_mode":"read-only",
            "live_spawn_receipt":{
                "status":"failed",
                "error":"unknown_agent_role",
                "tool":"wrong",
                "round_id":"wrong",
                "role":"wrong",
                "spawned_reviewer_agent_id":"wrong",
                "runtime_metadata":{"exposure":"unavailable"},
                "sandbox_mode":"workspace-write",
                "captured_at":"stale",
                "source_thread_id":"",
                "tool_invocation_id":""
            }
        }),
        "claim-falsifier",
        &mut failures,
    );
    assert!(has_fail(
        &failures,
        "review_round_live_spawn_unknown_agent_role"
    ));
    assert!(has_fail(&failures, "review_round_live_spawn_stale"));

    failures.clear();
    crate::review::round::spawn::receipts::spawn_receipt_errors(
        &receipt,
        &json!({
            "reviewer_agent_id":"agent-1",
            "runtime_metadata":{"exposure":"unavailable"},
            "sandbox_mode":"read-only",
            "live_spawn_receipt":{
                "status":"failed",
                "error":"timeout",
                "tool":"multi_agent_v1.spawn_agent",
                "round_id":"round-1",
                "role":"claim-falsifier",
                "spawned_reviewer_agent_id":"agent-1",
                "runtime_metadata":{"exposure":"unavailable"},
                "sandbox_mode":"read-only",
                "captured_at":"2026-06-26T09:00:00Z",
                "source_thread_id":"source-thread",
                "tool_invocation_id":"tool-call"
            }
        }),
        "claim-falsifier",
        &mut failures,
    );
    assert!(has_fail(&failures, "review_round_live_spawn_failed"));
    assert!(!has_fail(
        &failures,
        "review_round_live_spawn_unknown_agent_role"
    ));

    failures.clear();
    crate::review::round::spawn::receipts::spawn_receipt_errors(
        &receipt,
        &json!({
            "reviewer_agent_id":"agent-1",
            "runtime_metadata":{"exposure":"unavailable"},
            "sandbox_mode":"read-only",
            "live_spawn_receipt":{"status":"failed","error":"timeout"}
        }),
        "unknown-role",
        &mut failures,
    );
    assert!(failures.is_empty());
}
