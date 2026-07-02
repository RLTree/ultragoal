use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) fn print(root: &Path, path: &Path, value: &Value) {
    for line in lines(root, path, value) {
        println!("{line}");
    }
}

pub(crate) fn lines(root: &Path, path: &Path, value: &Value) -> Vec<String> {
    let mut lines = vec![summary(root, path, value)];
    if text(value, "status") != "pass" {
        lines.push(failure(root, path, value));
    }
    lines
}

fn summary(root: &Path, path: &Path, value: &Value) -> String {
    format!(
        "ultragoal-transaction-finalize {} proven={} operation=transaction_finalize candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        text(value, "status"),
        proven(value),
        text(value, "candidate_digest"),
        receipt_path(root, path, value),
        text(value, "run_id"),
        text(value, "correlation_id"),
        text(value, "claim_impact"),
        csv(value.pointer("/observability/supported_claims")),
        csv(value.pointer("/observability/blocked_claims"))
    )
}

fn failure(root: &Path, path: &Path, value: &Value) -> String {
    let run_id = text(value, "run_id");
    let metric_query =
        crate::cli::observe::query::bounded_metric_query_for_operation("transaction_finalize");
    format!(
        "failed_law={} failed_check={} why={} where={} claim_impact={} next_repair={} receipt={} run_id={} correlation_id={} query_logs='ultragoal observe logs query --run-id {} --limit 100' query_metrics='ultragoal observe metrics query --query '{}' --limit 100' query_traces='ultragoal observe traces query --run-id {} --limit 100'",
        text(value, "law_id"),
        text(value, "check_id"),
        text(value, "why_failed"),
        text(value, "where_failed"),
        text(value, "claim_impact"),
        text(value, "next_repair"),
        receipt_path(root, path, value),
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
        .unwrap_or("none")
        .to_string()
}

fn receipt_path(root: &Path, path: &Path, value: &Value) -> String {
    if let Some(path) = value
        .pointer("/receipt_observability_binding/command_receipt_path")
        .and_then(Value::as_str)
    {
        return path.to_string();
    }
    relative(root, path).unwrap_or_else(|| "<outside-root-receipt>".to_string())
}

fn relative(root: &Path, path: &Path) -> Option<String> {
    let root_abs = root.canonicalize().ok()?;
    let path_abs: PathBuf = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root_abs.join(path)
    };
    path_abs
        .strip_prefix(root_abs)
        .ok()
        .map(|rel| rel.to_string_lossy().into_owned())
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

#[cfg(test)]
mod tests;
