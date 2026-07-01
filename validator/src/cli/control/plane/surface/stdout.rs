use serde_json::Value;
use std::path::Path;

pub(super) fn print(path: &Path, value: &Value) {
    for line in lines(path, value) {
        println!("{line}");
    }
}

pub(crate) fn lines(path: &Path, value: &Value) -> Vec<String> {
    let mut lines = vec![summary(path, value)];
    if text(value, "status") != "pass" {
        lines.push(failure(path, value));
    }
    lines
}

fn summary(path: &Path, value: &Value) -> String {
    format!(
        "ultragoal-surface-audit {} proven={} operation={} candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        text(value, "status"),
        proven(value),
        text(value, "operation"),
        text(value, "candidate_digest"),
        path.display(),
        text(value, "run_id"),
        text(value, "correlation_id"),
        text(value, "claim_impact"),
        csv(value.pointer("/observability/supported_claims")),
        csv(value.pointer("/observability/blocked_claims"))
    )
}

fn failure(path: &Path, value: &Value) -> String {
    let run_id = text(value, "run_id");
    let metric_query =
        crate::cli::observe::query::bounded_metric_query_for_operation(text(value, "operation"));
    format!(
        "failed_law={} failed_check={} why={} where={} claim_impact={} next_repair={} receipt={} run_id={} correlation_id={} query_logs='ultragoal observe logs query --run-id {} --limit 100' query_metrics='ultragoal observe metrics query --query '{}' --limit 100' query_traces='ultragoal observe traces query --run-id {} --limit 100'",
        text(value, "law_id"),
        text(value, "check_id"),
        text(value, "why_failed"),
        text(value, "where_failed"),
        text(value, "claim_impact"),
        text(value, "next_repair"),
        path.display(),
        run_id,
        text(value, "correlation_id"),
        run_id,
        metric_query,
        run_id
    )
}

fn proven(value: &Value) -> String {
    if text(value, "status") != "pass" {
        return "none".to_string();
    }
    value
        .pointer("/observability/supported_claims")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .unwrap_or("package_surface_digest_alignment")
        .to_string()
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
