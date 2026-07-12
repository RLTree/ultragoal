use crate::review::round::ReviewFailure;
use serde_json::Value;

pub(crate) fn spawn_receipt_errors(
    receipt: &Value,
    row: &Value,
    role: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let Some(spawn) = row.get("live_spawn_receipt") else {
        out.push(failure("review_round_live_spawn_missing", role));
        return;
    };
    let Some(spec) = crate::review::round::config::review_role_spec(role) else {
        return;
    };
    if string(spawn, "status") != "spawned" {
        let code = if string(spawn, "error").contains("unknown_agent_role") {
            "review_round_live_spawn_unknown_agent_role"
        } else {
            "review_round_live_spawn_failed"
        };
        out.push(failure(code, role));
    }
    if string(spawn, "tool") != "multi_agent_v1.spawn_agent"
        || string(spawn, "round_id") != string(receipt, "round_id")
        || string(spawn, "role") != spec.role_name
        || string(spawn, "spawned_reviewer_agent_id") != string(row, "reviewer_agent_id")
        || string(spawn, "sandbox_mode") != string(row, "sandbox_mode")
        || !crate::review::round::runtime::metadata_matches(row, spawn)
        || string(spawn, "captured_at") != string(receipt, "generated_at")
        || string(spawn, "source_thread_id").is_empty()
        || string(spawn, "tool_invocation_id").is_empty()
    {
        out.push(failure("review_round_live_spawn_stale", role));
    }
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn failure(code: &str, detail: impl Into<String>) -> ReviewFailure {
    ReviewFailure::new("validator-execution-provenance", code, detail)
}
