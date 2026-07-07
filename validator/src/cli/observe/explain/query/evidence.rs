use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(in crate::cli::observe::explain) fn for_target(
    root: &Path,
    observed: Option<&Value>,
    candidate: &str,
) -> Value {
    let Some(event) = observed else {
        return empty();
    };
    let run_id = event.get("run_id").and_then(Value::as_str).unwrap_or("");
    let check_id = event.get("check_id").and_then(Value::as_str).unwrap_or("");
    let operation = event.get("operation").and_then(Value::as_str).unwrap_or("");
    let failure_class = event
        .get("failure_class")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let receipts = query_receipts(root);
    json!({
        "logs": matching_query(root, event, &receipts, "logs", candidate, run_id, check_id, operation, failure_class),
        "metrics": matching_query(root, event, &receipts, "metrics", candidate, run_id, check_id, operation, failure_class),
        "traces": matching_query(root, event, &receipts, "traces", candidate, run_id, check_id, operation, failure_class)
    })
}

fn empty() -> Value {
    json!({
        "logs": {"status":"missing"},
        "metrics": {"status":"missing"},
        "traces": {"status":"missing"}
    })
}

fn query_receipts(root: &Path) -> Vec<(PathBuf, Value)> {
    super::super::receipt::catalog::observability_json_files(root)
        .into_iter()
        .filter_map(|path| {
            let value = crate::json_boundary::read_json(&path).ok()?;
            (value.get("schema").and_then(Value::as_str)
                == Some(crate::cli::observe::types::QUERY_SCHEMA))
            .then_some((path, value))
        })
        .collect()
}

fn matching_query(
    root: &Path,
    event: &Value,
    receipts: &[(PathBuf, Value)],
    kind: &str,
    candidate: &str,
    run_id: &str,
    check_id: &str,
    operation: &str,
    failure_class: &str,
) -> Value {
    if let Some(value) = target_query_receipt(root, event, kind, candidate, run_id, failure_class) {
        return value;
    }
    receipts
        .iter()
        .rev()
        .find_map(|(path, value)| {
            query_matches(
                value,
                kind,
                candidate,
                run_id,
                check_id,
                operation,
                failure_class,
            )
            .then(|| query_summary(root, path, value))
        })
        .unwrap_or_else(|| json!({"status":"missing"}))
}

fn target_query_receipt(
    root: &Path,
    event: &Value,
    kind: &str,
    candidate: &str,
    run_id: &str,
    _failure_class: &str,
) -> Option<Value> {
    let receipt = event.get("receipt_path").and_then(Value::as_str)?;
    let path = crate::output_path::claim_artifact_path(
        root,
        Path::new(receipt),
        "observe query evidence receipt",
    )
    .ok()?;
    let value = crate::json_boundary::read_json(&path).ok()?;
    if value.get("schema").and_then(Value::as_str) != Some(crate::cli::observe::types::QUERY_SCHEMA)
        || value.get("query_kind").and_then(Value::as_str) != Some(kind)
        || value.get("candidate_digest").and_then(Value::as_str) != Some(candidate)
        || value.get("run_id").and_then(Value::as_str) != Some(run_id)
    {
        return None;
    }
    Some(query_summary(root, &path, &value))
}

fn query_summary(root: &Path, path: &Path, value: &Value) -> Value {
    let path = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    json!({
        "status": value.get("status").cloned().unwrap_or(json!("unknown")),
        "path": path,
        "row_count": value.get("row_count").cloned().unwrap_or(json!(0)),
        "why_failed": value.get("observed_why_failed")
            .or_else(|| value.get("why_failed"))
            .cloned()
            .unwrap_or(json!("none")),
        "where_failed": value.get("observed_where_failed")
            .or_else(|| value.get("where_failed"))
            .cloned()
            .unwrap_or(json!("none")),
        "next_repair": value.get("observed_next_repair")
            .or_else(|| value.get("next_repair"))
            .cloned()
            .unwrap_or(json!("none")),
        "observed_failure_class": value.get("observed_failure_class")
            .or_else(|| value.get("failure_class"))
            .cloned()
            .unwrap_or(json!("none")),
        "metric_failure_class": value.get("metric_failure_class").cloned().unwrap_or(json!("none")),
        "metric_error_count": value.get("metric_error_count").cloned().unwrap_or(json!(0))
    })
}

fn query_matches(
    value: &Value,
    kind: &str,
    candidate: &str,
    run_id: &str,
    check_id: &str,
    operation: &str,
    failure_class: &str,
) -> bool {
    let same_run = value.get("run_id").and_then(Value::as_str) == Some(run_id);
    value.get("query_kind").and_then(Value::as_str) == Some(kind)
        && value.get("candidate_digest").and_then(Value::as_str) == Some(candidate)
        && (same_run || query_mentions(value, check_id) || query_mentions(value, operation))
        && (same_run || query_matches_failure(value, kind, failure_class))
}

fn query_matches_failure(value: &Value, kind: &str, failure_class: &str) -> bool {
    if failure_class.is_empty() || failure_class == "none" {
        return true;
    }
    if kind == "metrics" {
        return (value.get("metric_failure_class").and_then(Value::as_str) == Some(failure_class)
            && value
                .get("metric_error_count")
                .and_then(Value::as_u64)
                .is_some_and(|count| count > 0))
            || value.get("failure_class").and_then(Value::as_str) == Some(failure_class);
    }
    value
        .get("observed_failure_class")
        .or_else(|| value.get("failure_class"))
        .and_then(Value::as_str)
        == Some(failure_class)
}

fn query_mentions(value: &Value, needle: &str) -> bool {
    !needle.is_empty()
        && value
            .get("query")
            .and_then(Value::as_str)
            .is_some_and(|query| query.contains(needle))
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;
