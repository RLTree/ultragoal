use super::LiveLoopCommand;
use serde_json::Value;

pub(super) fn print_run_summary(command: &LiveLoopCommand, receipt: &Value, blocker: &Value) {
    println!("{}", summary_line(command, receipt, blocker));
}

pub(super) fn summary_line(command: &LiveLoopCommand, receipt: &Value, blocker: &Value) -> String {
    format!(
        "ultragoal-loop {} candidate={} tier={} cache_mode={} duration_ms={} worker_count={} task_count={} queue_depth={} changed_file_count={} affected_node_count={} unaffected_node_count={} critical_path='{}' first_blocker={} first_product_blocker={} first_observability_blocker={} first_speed_blocker={} first_control_board_blocker={} why={} next_repair={} narrow_rerun='{}' broad_rerun='{}' claim_ceiling='{}' receipt={} run_id={} correlation_id={} trace_id={} span_id={} query_logs='{}' query_metrics='{}' query_traces='{}'",
        text(receipt, "status", "fail"),
        text(receipt, "candidate_digest", "<missing>"),
        command.tier,
        command.cache_mode,
        number(receipt, "duration_ms"),
        number(receipt, "worker_count"),
        number(receipt, "task_count"),
        number(receipt, "queue_depth"),
        nested_nested_number(
            receipt,
            "audit_context",
            "changed_inputs",
            "changed_file_count"
        ),
        nested_nested_number(
            receipt,
            "audit_context",
            "changed_inputs",
            "affected_node_count"
        ),
        nested_nested_number(
            receipt,
            "audit_context",
            "changed_inputs",
            "unaffected_node_count"
        ),
        text(receipt, "critical_path", "unknown"),
        text(blocker, "id", "unknown"),
        nested_text(receipt, "first_product_blocker", "id", "none"),
        nested_text(receipt, "first_observability_blocker", "id", "none"),
        nested_text(receipt, "first_speed_blocker", "id", "none"),
        nested_text(receipt, "first_control_board_blocker", "id", "none"),
        text(blocker, "why_failed", "unknown"),
        text(blocker, "next_repair", "unknown"),
        text(blocker, "narrow_rerun", "unknown"),
        text(blocker, "broad_rerun", "unknown"),
        text(receipt, "claim_ceiling", "source-local only"),
        command.receipt.display(),
        text(&receipt["observability"], "run_id", "unknown"),
        text(&receipt["observability"], "correlation_id", "unknown"),
        text(&receipt["observability"], "trace_id", "unknown"),
        text(&receipt["observability"]["trace"], "span_id", "unknown"),
        query_example(receipt, 0),
        query_example(receipt, 1),
        query_example(receipt, 2)
    )
}

fn query_example(receipt: &Value, index: usize) -> &str {
    receipt
        .get("observability")
        .and_then(|value| value.get("query_examples"))
        .and_then(Value::as_array)
        .and_then(|examples| examples.get(index))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
}

fn text<'a>(value: &'a Value, key: &str, default: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn nested_text<'a>(value: &'a Value, object: &str, key: &str, default: &'a str) -> &'a str {
    value
        .get(object)
        .and_then(|nested| nested.get(key))
        .and_then(Value::as_str)
        .unwrap_or(default)
}

fn nested_nested_number(value: &Value, object: &str, nested: &str, key: &str) -> u64 {
    value
        .get(object)
        .and_then(|value| value.get(nested))
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}
