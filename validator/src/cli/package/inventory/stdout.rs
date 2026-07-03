use serde_json::Value;

pub(super) fn print(value: &Value, failures: &[String]) {
    for line in contract(value, failures) {
        println!("{line}");
    }
}

pub(super) fn contract(value: &Value, failures: &[String]) -> Vec<String> {
    let status = text(value, "status");
    let run_id = text(value, "run_id");
    let receipt = text(value, "receipt_path");
    let correlation_id = text(value, "correlation_id");
    let metric_query =
        crate::cli::observe::query::bounded_metric_query_for_operation(text(value, "operation"));
    let claim_impact = text(value, "claim_impact");
    let mut lines = vec![format!(
        "ultragoal-package-inventory {status} operation={} candidate={} receipt={receipt} run_id={run_id} correlation_id={correlation_id} claim_impact={claim_impact} supported_claims={} unsupported_claims={}",
        text(value, "operation"),
        text(value, "candidate_digest"),
        csv(value.get("supported_claims")),
        csv(value.get("blocked_claims"))
    )];
    if status != "pass" {
        lines.push(format!(
            "failed_law={} failed_check={} why={} first_failure={} failure_count={} where={} claim_impact={claim_impact} next_repair={} receipt={receipt} run_id={run_id} correlation_id={correlation_id} query_logs='ultragoal observe logs query --run-id {run_id} --limit 100' query_metrics='ultragoal observe metrics query --query '{metric_query}' --limit 100' query_traces='ultragoal observe traces query --run-id {run_id} --limit 100'",
            text(value, "law_id"),
            text(value, "check_id"),
            text(value, "why_failed"),
            failures.first().map(String::as_str).unwrap_or("none"),
            failures.len(),
            text(value, "where_failed"),
            text(value, "next_repair")
        ));
    }
    lines
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
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| "none".to_string())
}
