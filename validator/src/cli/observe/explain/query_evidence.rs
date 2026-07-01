use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(super) fn for_target(root: &Path, observed: Option<&Value>, candidate: &str) -> Value {
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
        "logs": matching_query(&receipts, "logs", candidate, run_id, check_id, operation, failure_class),
        "metrics": matching_query(&receipts, "metrics", candidate, run_id, check_id, operation, failure_class),
        "traces": matching_query(&receipts, "traces", candidate, run_id, check_id, operation, failure_class)
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
    let dir = root.join("validation_artifacts/observability");
    let mut paths = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
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
    receipts: &[(PathBuf, Value)],
    kind: &str,
    candidate: &str,
    run_id: &str,
    check_id: &str,
    operation: &str,
    failure_class: &str,
) -> Value {
    receipts
        .iter()
        .rev()
        .find_map(|(path, value)| {
            let path = path.to_string_lossy().to_string();
            query_matches(value, kind, candidate, run_id, check_id, operation, failure_class).then(|| {
                json!({
                    "status": value.get("status").cloned().unwrap_or(json!("unknown")),
                    "path": path,
                    "row_count": value.get("row_count").cloned().unwrap_or(json!(0)),
                    "why_failed": value.get("why_failed").cloned().unwrap_or(json!("none")),
                    "where_failed": value.get("where_failed").cloned().unwrap_or(json!("none")),
                    "observed_failure_class": value.get("observed_failure_class").cloned().unwrap_or(json!("none")),
                    "metric_failure_class": value.get("metric_failure_class").cloned().unwrap_or(json!("none")),
                    "metric_error_count": value.get("metric_error_count").cloned().unwrap_or(json!(0))
                })
            })
        })
        .unwrap_or_else(|| json!({"status":"missing"}))
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
    value.get("query_kind").and_then(Value::as_str) == Some(kind)
        && value.get("candidate_digest").and_then(Value::as_str) == Some(candidate)
        && (value.get("run_id").and_then(Value::as_str) == Some(run_id)
            || query_mentions(value, check_id)
            || query_mentions(value, operation))
        && query_matches_failure(value, kind, failure_class)
}

fn query_matches_failure(value: &Value, kind: &str, failure_class: &str) -> bool {
    if failure_class.is_empty() || failure_class == "none" {
        return true;
    }
    if kind == "metrics" {
        return value.get("metric_failure_class").and_then(Value::as_str) == Some(failure_class)
            && value
                .get("metric_error_count")
                .and_then(Value::as_u64)
                .is_some_and(|count| count > 0);
    }
    value.get("observed_failure_class").and_then(Value::as_str) == Some(failure_class)
}

fn query_mentions(value: &Value, needle: &str) -> bool {
    !needle.is_empty()
        && value
            .get("query")
            .and_then(Value::as_str)
            .is_some_and(|query| query.contains(needle))
}
