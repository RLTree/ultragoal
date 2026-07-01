use serde_json::Value;

pub(super) fn print_summary(value: &Value, review_target_digest: &str) {
    println!(
        "ultragoal-review-target-build {} proven={} review_target_digest={} candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        text(value, "status"),
        if text(value, "status") == "pass" {
            super::review_target::CLAIM_ID
        } else {
            "none"
        },
        review_target_digest,
        text(value, "candidate_digest"),
        text(value, "receipt_path"),
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
    let metric_query = crate::cli::observe::query::bounded_metric_query_for_operation(
        super::review_target::OPERATION,
    );
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
