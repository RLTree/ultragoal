use serde_json::{Value, json};
use std::path::Path;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(crate) struct NextActionCommand {
    pub(crate) json: bool,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<NextActionCommand>, String> {
    if raw.first().map(String::as_str) != Some("next") {
        return Ok(None);
    }
    Ok(Some(NextActionCommand {
        json: raw.iter().any(|arg| arg == "--json"),
    }))
}

pub(crate) fn run(root: &Path, command: &NextActionCommand) -> Result<i32, String> {
    let state = crate::cli::current_state::snapshot(root)?;
    let explain_command = crate::cli::observe::command::ObserveCommand {
        operation: crate::cli::observe::command::ObserveOperation::ExplainNext,
        receipt: Some("validation_artifacts/observability/observe-explain-next.json".into()),
        query: None,
        run_id: None,
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 262_144,
        timeout_ms: 30_000,
    };
    let explain = crate::cli::observe::explain::build_next_plan(root, &explain_command, &state)?;
    let action = next_action(&state, &explain);
    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&action).expect("json value")
        );
    } else {
        print_summary(&action);
    }
    Ok(i32::from(
        action.get("status").and_then(Value::as_str) != Some("pass"),
    ))
}

pub(crate) fn next_action(state: &Value, explain: &Value) -> Value {
    let blocker = state.get("first_blocker").unwrap_or(&Value::Null);
    let explanation = explain.get("explanation").unwrap_or(&Value::Null);
    let status = if text(blocker, "id", "unknown") == "none" {
        "pass"
    } else {
        "fail"
    };
    json!({
        "schema": "harness-ultragoal.next-action.v1",
        "status": status,
        "candidate_digest": text(state, "candidate_digest", "missing"),
        "dirty_state": state.get("git").cloned().unwrap_or_else(|| json!({"dirty": true})),
        "active_stage": text(state, "active_stage", "custom_tooling_prerequisite"),
        "first_legal_blocker": blocker,
        "root_cause": text(explanation, "root_cause", text(blocker, "why_failed", "unknown")),
        "implicated_artifacts": explanation
            .get("implicated_paths")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "blocked_claim": text(explain, "claim_impact", "source_local_custom_tooling_prerequisite_blocked"),
        "smallest_repair": text(explanation, "smallest_repair", text(state, "next_repair", "repair typed authority source")),
        "narrow_rerun": text(explanation, "narrow_rerun", text(state, "narrow_rerun", "ultragoal current-state --json")),
        "allowed_broad_rerun": text(explanation, "broad_rerun", "source audit once after narrow proof passes"),
        "forbidden_actions": explain.get("forbidden_actions").cloned().unwrap_or_else(|| json!([
            "do not refresh install/cache",
            "do not finalize final packet",
            "do not claim readiness release completion",
            "do not call update_goal"
        ])),
        "claim_ceiling": text(state, "claim_ceiling", "source_local_custom_tooling_prerequisite_only"),
        "authority_sources": {
            "current_state": "ultragoal current-state --json",
            "observe_explain": "ultragoal observe explain --next",
            "source_receipts": state.get("source_receipts").cloned().unwrap_or_else(|| json!([]))
        },
        "supported_claims": ["first_legal_source_local_repair_move_only"],
        "blocked_claims": explain.get("blocked_claims").cloned().unwrap_or_else(|| json!([
            "observability_speed_or_command_roundtrip",
            "readiness",
            "release",
            "completion",
            "final_packet_correctness",
            "install_cache_refresh",
            "registry_reviewer_exposure",
            "update_goal_eligibility"
        ]))
    })
}

fn print_summary(action: &Value) {
    println!(
        "ultragoal-next {} candidate={} blocker={} root_cause={} repair={} narrow_rerun='{}' broad_rerun='{}' claim_ceiling='{}'",
        text(action, "status", "fail"),
        text(action, "candidate_digest", "missing"),
        text(
            action.get("first_legal_blocker").unwrap_or(&Value::Null),
            "id",
            "unknown"
        ),
        text(action, "root_cause", "unknown"),
        text(action, "smallest_repair", "unknown"),
        text(action, "narrow_rerun", "unknown"),
        text(action, "allowed_broad_rerun", "unknown"),
        text(action, "claim_ceiling", "unknown")
    );
}

fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}
