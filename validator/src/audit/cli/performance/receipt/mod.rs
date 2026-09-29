use serde_json::Value;

mod node_speed_evidence;

const SCHEMA: &str = "harness-ultragoal.cli-performance-receipt.v1";
const SPEED_NODE_CLAIM_CEILING: &str = "source_local_speed_node_timing_only";

pub(crate) fn surface_value_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push("cli_performance_receipt_wrong_schema".to_string());
    }
    let status = value.get("status").and_then(Value::as_str);
    for ptr in [
        "/command/argv",
        "/budget/class",
        "/digests/candidate",
        "/cache/mode",
        "/concurrency/worker_count",
        "/concurrency/queue_depth",
        "/concurrency/isolation_namespace",
        "/telemetry/wall_clock_ms",
        "/telemetry/cpu_ms",
        "/telemetry/peak_memory_bytes",
        "/telemetry/io_bytes",
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
        && value.get("claim_ceiling").and_then(Value::as_str) != Some(SPEED_NODE_CLAIM_CEILING)
    {
        out.push("cli_performance_pass_without_speed_node_claim_ceiling".to_string());
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
    if status == Some("pass") {
        out.extend(node_speed_evidence::speed_failures(value, None));
    }
    out
}

fn check_budget_authority(value: &Value, out: &mut Vec<String>) {
    let Some(raw_class) = value.pointer("/budget/class").and_then(Value::as_str) else {
        return;
    };
    let Some((cold_p95_ms, hard_ceiling_ms)) = budget_limits(raw_class) else {
        out.push(format!(
            "cli_performance_receipt_noncanonical_budget_class:{raw_class}"
        ));
        return;
    };
    for (ptr, expected, code) in [
        (
            "/budget/cold_p95_ms",
            cold_p95_ms,
            "cli_performance_receipt_budget_cold_p95_mismatch",
        ),
        (
            "/budget/target_ms",
            cold_p95_ms,
            "cli_performance_receipt_budget_target_mismatch",
        ),
        (
            "/budget/threshold_ms",
            cold_p95_ms,
            "cli_performance_receipt_budget_threshold_mismatch",
        ),
        (
            "/budget/hard_ceiling_ms",
            hard_ceiling_ms,
            "cli_performance_receipt_budget_hard_ceiling_mismatch",
        ),
    ] {
        if value.pointer(ptr).and_then(Value::as_u64) != Some(expected) {
            out.push(format!("{code}:{raw_class}"));
        }
    }
}

fn budget_limits(class: &str) -> Option<(u64, u64)> {
    match class {
        "hot_edit_check" => Some((5_000, 5_000)),
        "focused_repair" => Some((15_000, 15_000)),
        "standard_source_local" => Some((30_000, 60_000)),
        "strict_local" | "strict_fixtures" | "strict_coverage" | "strict_final" => {
            Some((60_000, 180_000))
        }
        "external_live" => Some((30_000, 30_000)),
        _ => None,
    }
}

pub(crate) fn same_candidate_pass_failures(value: &Value, expected_candidate: &str) -> Vec<String> {
    let mut out = surface_value_failures(value);
    out.push("cli_performance_receipt_independent_execution_unavailable".to_string());
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
    if value
        .pointer("/telemetry/wall_clock_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
    {
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
    let worker_count = value
        .pointer("/concurrency/worker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if worker_count == 0 || worker_count > 256 {
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
    out.extend(node_speed_evidence::speed_failures(
        value,
        Some(expected_candidate),
    ));
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn repository_telemetry_cannot_prove_execution() {
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('a');
        let failures = super::same_candidate_pass_failures(
            &json!({
                "status": "pass",
                "digests": {"candidate": candidate}
            }),
            &candidate,
        );
        assert!(
            failures
                .contains(&"cli_performance_receipt_independent_execution_unavailable".to_string()),
            "{failures:?}"
        );
    }
}
