use crate::cli::performance::types::{BudgetClass, PERFORMANCE_RECEIPT_SCHEMA};
use serde_json::Value;

pub(crate) fn surface_value_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(PERFORMANCE_RECEIPT_SCHEMA) {
        out.push("cli_performance_receipt_wrong_schema".to_string());
    }
    let status = value.get("status").and_then(Value::as_str);
    for ptr in [
        "/command/argv",
        "/budget/class",
        "/digests/candidate",
        "/cache/mode",
        "/concurrency/worker_count",
        "/telemetry/wall_clock_ms",
    ] {
        if value.pointer(ptr).is_none() {
            out.push(format!("cli_performance_receipt_missing:{ptr}"));
        }
    }
    check_budget_authority(value, &mut out);
    if status == Some("fail") && value.pointer("/failure/check_id").is_none() {
        out.push("cli_performance_receipt_missing:/failure/check_id".to_string());
    }
    if status == Some("pass")
        && value.get("claim_ceiling").and_then(Value::as_str) != Some("performance_proven")
    {
        out.push("cli_performance_pass_without_positive_claim_ceiling".to_string());
    }
    if status == Some("pass")
        && value
            .get("blocked_claim_classes")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty())
    {
        out.push("cli_performance_pass_with_blocked_claims".to_string());
    }
    let blocked = value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(|rows| !rows.is_empty());
    if status == Some("fail") && !blocked {
        out.push("cli_performance_fail_without_blocked_claims".to_string());
    }
    out
}

fn check_budget_authority(value: &Value, out: &mut Vec<String>) {
    let Some(raw_class) = value.pointer("/budget/class").and_then(Value::as_str) else {
        return;
    };
    let Some(class) = BudgetClass::from_str(raw_class).filter(|class| class.id() == raw_class)
    else {
        out.push(format!(
            "cli_performance_receipt_noncanonical_budget_class:{raw_class}"
        ));
        return;
    };
    let fields = [
        (
            "/budget/cold_p95_ms",
            class.cold_p95_ms(),
            "cli_performance_receipt_budget_cold_p95_mismatch",
        ),
        (
            "/budget/target_ms",
            class.cold_p95_ms(),
            "cli_performance_receipt_budget_target_mismatch",
        ),
        (
            "/budget/threshold_ms",
            class.cold_p95_ms(),
            "cli_performance_receipt_budget_threshold_mismatch",
        ),
        (
            "/budget/hard_ceiling_ms",
            class.hard_ceiling_ms(),
            "cli_performance_receipt_budget_hard_ceiling_mismatch",
        ),
    ];
    for (ptr, expected, code) in fields {
        if value.pointer(ptr).and_then(Value::as_u64) != Some(expected) {
            out.push(format!("{code}:{raw_class}"));
        }
    }
}

pub(crate) fn same_candidate_pass_failures(value: &Value, expected_candidate: &str) -> Vec<String> {
    let mut out = surface_value_failures(value);
    let candidate = value
        .pointer("/digests/candidate")
        .and_then(Value::as_str)
        .unwrap_or("");
    if candidate != expected_candidate {
        out.push(format!(
            "cli_performance_receipt_candidate_digest_mismatch:{candidate}!={expected_candidate}"
        ));
    }
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("cli_performance_receipt_not_pass".to_string());
    }
    if value
        .get("failure")
        .is_some_and(|failure| failure != &Value::Null)
    {
        out.push("cli_performance_pass_has_failure".to_string());
    }
    let wall_ms = value
        .pointer("/telemetry/wall_clock_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if wall_ms == 0 {
        out.push("cli_performance_receipt_placeholder_wall_clock".to_string());
    }
    if value
        .pointer("/performance_regression/status")
        .and_then(Value::as_str)
        != Some("pass")
    {
        out.push("cli_performance_receipt_regression_not_pass".to_string());
    }
    if value.pointer("/cache/mode").and_then(Value::as_str) != Some("disabled")
        || value
            .pointer("/cache/no_cache_mode_result")
            .and_then(Value::as_str)
            != Some("executed_without_cache")
    {
        out.push("cli_performance_receipt_cache_honesty_missing".to_string());
    }
    if value
        .pointer("/concurrency/worker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
    {
        out.push("cli_performance_receipt_unbounded_concurrency".to_string());
    }
    if value
        .get("supported_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter()
                .any(|row| row.as_str() == Some("update_goal_eligibility"))
        })
    {
        out.push("cli_performance_receipt_update_goal_overclaim".to_string());
    }
    out
}
