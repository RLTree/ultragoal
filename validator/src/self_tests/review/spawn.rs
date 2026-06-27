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
        "contract_claim_falsifier",
        &mut failures,
    );
    assert!(has_fail(&failures, "review_round_live_spawn_missing"));

    failures.clear();
    crate::review::round::spawn::receipts::spawn_receipt_errors(
        &receipt,
        &json!({
            "reviewer_agent_id":"agent-1",
            "model":"gpt-5",
            "reasoning_effort":"high",
            "live_spawn_receipt":{
                "status":"failed",
                "error":"unknown_agent_type",
                "tool":"wrong",
                "round_id":"wrong",
                "agent_type":"wrong",
                "spawned_reviewer_agent_id":"wrong",
                "model":"wrong",
                "reasoning_effort":"low",
                "captured_at":"stale",
                "source_thread_id":"",
                "tool_invocation_id":""
            }
        }),
        "contract_claim_falsifier",
        &mut failures,
    );
    assert!(has_fail(
        &failures,
        "review_round_live_spawn_unknown_agent_type"
    ));
    assert!(has_fail(&failures, "review_round_live_spawn_stale"));

    failures.clear();
    crate::review::round::spawn::receipts::spawn_receipt_errors(
        &receipt,
        &json!({
            "reviewer_agent_id":"agent-1",
            "model":"gpt-5",
            "reasoning_effort":"high",
            "live_spawn_receipt":{
                "status":"failed",
                "error":"timeout",
                "tool":"multi_agent_v1.spawn_agent",
                "round_id":"round-1",
                "agent_type":"harness_contract_claim_falsifier",
                "spawned_reviewer_agent_id":"agent-1",
                "model":"gpt-5",
                "reasoning_effort":"high",
                "captured_at":"2026-06-26T09:00:00Z",
                "source_thread_id":"source-thread",
                "tool_invocation_id":"tool-call"
            }
        }),
        "contract_claim_falsifier",
        &mut failures,
    );
    assert!(has_fail(&failures, "review_round_live_spawn_failed"));
    assert!(!has_fail(
        &failures,
        "review_round_live_spawn_unknown_agent_type"
    ));

    failures.clear();
    crate::review::round::spawn::receipts::spawn_receipt_errors(
        &receipt,
        &json!({
            "reviewer_agent_id":"agent-1",
            "model":"gpt-5",
            "reasoning_effort":"high",
            "live_spawn_receipt":{"status":"failed","error":"timeout"}
        }),
        "unknown_persona",
        &mut failures,
    );
    assert!(failures.is_empty());
}
