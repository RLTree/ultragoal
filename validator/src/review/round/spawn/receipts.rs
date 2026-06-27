use crate::review::round::ReviewFailure;
use serde_json::Value;

pub(crate) fn spawn_receipt_errors(
    receipt: &Value,
    row: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let Some(spawn) = row.get("live_spawn_receipt") else {
        out.push(failure("review_round_live_spawn_missing", persona));
        return;
    };
    let Some(spec) = crate::review::round::config::persona_spec(persona) else {
        return;
    };
    if string(spawn, "status") != "spawned" {
        let code = if string(spawn, "error").contains("unknown_agent_type") {
            "review_round_live_spawn_unknown_agent_type"
        } else {
            "review_round_live_spawn_failed"
        };
        out.push(failure(code, persona));
    }
    if string(spawn, "tool") != "multi_agent_v1.spawn_agent"
        || string(spawn, "round_id") != string(receipt, "round_id")
        || string(spawn, "agent_type") != spec.agent_type
        || string(spawn, "spawned_reviewer_agent_id") != string(row, "reviewer_agent_id")
        || string(spawn, "model") != string(row, "model")
        || string(spawn, "reasoning_effort") != string(row, "reasoning_effort")
        || string(spawn, "captured_at") != string(receipt, "generated_at")
        || string(spawn, "source_thread_id").is_empty()
        || string(spawn, "tool_invocation_id").is_empty()
    {
        out.push(failure("review_round_live_spawn_stale", persona));
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
