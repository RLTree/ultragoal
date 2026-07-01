use serde_json::Value;

pub(super) fn print_summary(value: &Value) {
    println!(
        "ultragoal-review-round-verify {} operation={} candidate={} receipt={} review_round_receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        text(value, "status"),
        text(value, "operation"),
        text(value, "candidate_digest"),
        text(value, "receipt_path"),
        review_receipt(value),
        text(value, "run_id"),
        text(value, "correlation_id"),
        text(value, "claim_impact"),
        csv(value.get("supported_claims")),
        csv(value.get("blocked_claims"))
    );
    if text(value, "status") != "pass" {
        print_failure(value);
    }
}

fn print_failure(value: &Value) {
    let run_id = text(value, "run_id");
    let metric_query =
        crate::cli::observe::query::bounded_metric_query_for_operation(super::round::OPERATION);
    println!(
        "failed_law={} failed_check={} why={} where={} claim_impact={} next_repair={} receipt={} run_id={} correlation_id={} query_logs='ultragoal observe logs query --run-id {} --limit 100' query_metrics='ultragoal observe metrics query --query '{}' --limit 100' query_traces='ultragoal observe traces query --run-id {} --limit 100'",
        text(value, "law_id"),
        text(value, "check_id"),
        text(value, "why_failed"),
        text(value, "where_failed"),
        text(value, "claim_impact"),
        text(value, "next_repair"),
        text(value, "receipt_path"),
        run_id,
        text(value, "correlation_id"),
        run_id,
        metric_query,
        run_id
    );
}

fn review_receipt(value: &Value) -> &str {
    value
        .get("event")
        .and_then(|event| event.get("artifact_path"))
        .and_then(Value::as_str)
        .and_then(|items| items.split(',').next())
        .unwrap_or("<missing>")
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("<missing>")
}

fn csv(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "none".to_string())
}
