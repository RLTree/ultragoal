use serde_json::{Value, json};
use std::path::Path;

pub(in crate::cli::observe::explain) fn latest_failed_event(
    root: &Path,
    field: &str,
    expected: &str,
) -> Option<Value> {
    super::super::receipt::catalog::observability_json_files(root)
        .into_iter()
        .rev()
        .find_map(|path| {
            let value = crate::json_boundary::read_json(&path).ok()?;
            (is_query_receipt(&value)
                && value.get(field).and_then(Value::as_str) == Some(expected)
                && value.get("status").and_then(Value::as_str) == Some("fail"))
            .then(|| event_from_receipt(root, &path, &value))
        })
}

fn is_query_receipt(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::QUERY_SCHEMA)
}

fn event_from_receipt(root: &Path, path: &Path, value: &Value) -> Value {
    let receipt_path = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let get = |key: &str| value.get(key).cloned().unwrap_or(Value::Null);
    let query_kind = value
        .get("query_kind")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    json!({
        "run_id": get("run_id"),
        "correlation_id": get("correlation_id"),
        "trace_id": get("trace_id"),
        "candidate_digest": get("candidate_digest"),
        "operation": format!("observe.{query_kind}.query"),
        "status": get("status"),
        "failure_class": failure_class(value),
        "why_failed": failure_text(value, &[
            "observed_why_failed",
            "why_failed",
            "failure",
        ]),
        "where_failed": failure_text(value, &[
            "observed_where_failed",
            "where_failed",
        ]),
        "next_repair": failure_text(value, &[
            "observed_next_repair",
            "next_repair",
        ]),
        "claim_impact": failure_text(value, &[
            "observed_claim_impact",
            "claim_impact",
        ]),
        "receipt_path": receipt_path.clone(),
        "artifact_path": receipt_path,
        "law_id": get("law_id"),
        "check_id": get("check_id"),
        "claim_id": get("claim_id"),
        "duration_ms": get("timeout_ms"),
        "worker_count": 1,
        "task_count": get("metric_task_count"),
        "queue_depth": get("metric_queue_depth"),
        "query_kind": query_kind,
        "row_count": get("row_count"),
        "result_digest": get("result_digest")
    })
}

fn failure_class(value: &Value) -> Value {
    for key in [
        "observed_failure_class",
        "metric_failure_class",
        "failure_class",
    ] {
        if value
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty() && text != "none")
        {
            return value[key].clone();
        }
    }
    json!("observability_query_reconciliation_failed")
}

fn failure_text(value: &Value, keys: &[&str]) -> Value {
    for key in keys {
        if value
            .get(*key)
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty() && text != "none")
        {
            return value[*key].clone();
        }
    }
    json!("observability query receipt failed without specific failure text")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_target_prefers_specific_failure_fields_before_generic_fallbacks() {
        let metric = json!({
            "observed_failure_class": "none",
            "metric_failure_class": "coverage_prove_failure",
            "failure_class": "generic_failure",
            "observed_why_failed": "none",
            "why_failed": "coverage digest mismatch"
        });

        assert_eq!(failure_class(&metric), json!("coverage_prove_failure"));
        assert_eq!(
            failure_text(&metric, &["observed_why_failed", "why_failed"]),
            json!("coverage digest mismatch")
        );
    }

    #[test]
    fn receipt_target_fails_closed_when_query_receipt_has_no_specific_failure_text() {
        let opaque = json!({
            "failure_class": "none",
            "why_failed": "none",
            "next_repair": ""
        });

        assert_eq!(
            failure_class(&opaque),
            json!("observability_query_reconciliation_failed")
        );
        assert_eq!(
            failure_text(&opaque, &["why_failed", "next_repair"]),
            json!("observability query receipt failed without specific failure text")
        );
    }
}
