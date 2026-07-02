use serde_json::Value;
use std::path::Path;

pub(crate) fn print(path: &Path, value: &Value) {
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
        "{} {} proven={} operation={} candidate={} receipt={} registry_receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        prefix(value),
        text(value, "status"),
        proven(value),
        text(value, "operation"),
        text(value, "candidate_digest"),
        receipt_path(path, value),
        registry_receipt(value),
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
        "failed_law={} failed_check={} why={} where={} claim_impact={} next_repair={} receipt={} registry_receipt={} run_id={} correlation_id={} query_logs='ultragoal observe logs query --run-id {} --limit 100' query_metrics='ultragoal observe metrics query --query '{}' --limit 100' query_traces='ultragoal observe traces query --run-id {} --limit 100'",
        text(value, "law_id"),
        text(value, "check_id"),
        bounded_text(value, "why_failed"),
        bounded_text(value, "where_failed"),
        text(value, "claim_impact"),
        bounded_text(value, "next_repair"),
        receipt_path(path, value),
        registry_receipt(value),
        run_id,
        text(value, "correlation_id"),
        run_id,
        metric_query,
        run_id
    )
}

fn bounded_text(value: &Value, field: &str) -> String {
    let text = text(value, field);
    let mut parts = text.split(" | ");
    let mut out = Vec::new();
    for part in parts.by_ref().take(6) {
        out.push(part);
    }
    let suffix = if parts.next().is_some() {
        " | ... see receipt/explain for full failure graph"
    } else {
        ""
    };
    let joined = out.join(" | ");
    if joined.chars().count() > 700 {
        format!(
            "{}... see receipt/explain for full failure graph",
            joined.chars().take(700).collect::<String>()
        )
    } else {
        format!("{joined}{suffix}")
    }
}

fn prefix(value: &Value) -> &'static str {
    match text(value, "operation") {
        "registry_probe" | "app_surface_probe" => "ultragoal-registry-probe",
        _ => "ultragoal-control",
    }
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
        .unwrap_or("none")
        .to_string()
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("<missing>")
}

fn receipt_path<'a>(path: &'a Path, value: &'a Value) -> String {
    value
        .pointer("/receipt_observability_binding/command_receipt_path")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn registry_receipt(value: &Value) -> String {
    value
        .pointer("/receipt_observability_binding/registry_receipt_path")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string()
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

#[cfg(test)]
mod tests;
