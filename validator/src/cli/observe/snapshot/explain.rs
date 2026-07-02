use crate::cli::observe::types::RECEIPT_SCHEMA;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn evidence(root: &Path, candidate: &str, event: &Value) -> Value {
    let run_id = event.get("run_id").and_then(Value::as_str).unwrap_or("");
    let operation = event.get("operation").and_then(Value::as_str).unwrap_or("");
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
        .rev()
        .filter_map(|path| {
            let value = crate::json_boundary::read_json(&path).ok()?;
            let path_text = super::target::relative_path_for_root(root, &path);
            explain_matches(&value, candidate, run_id, operation).then(|| {
                json!({
                    "status": value.get("status").cloned().unwrap_or(json!("unknown")),
                    "path": path_text,
                    "fallback_used": value
                        .pointer("/explanation/fallback_used")
                        .cloned()
                        .unwrap_or(json!(true)),
                    "root_cause": value
                        .pointer("/explanation/root_cause")
                        .cloned()
                        .unwrap_or(json!("unknown")),
                    "target_status": value
                        .pointer("/observed_run/status")
                        .cloned()
                        .unwrap_or(json!("unknown")),
                    "where_failed": observed_or_query(
                        &value,
                        "observed_where_failed",
                        "/explanation/query_evidence/logs/where_failed",
                    ),
                    "why_failed": observed_or_query(
                        &value,
                        "observed_why_failed",
                        "/explanation/query_evidence/logs/why_failed",
                    ),
                    "implicated_paths": value
                        .pointer("/explanation/implicated_paths")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "smallest_repair": value
                        .pointer("/explanation/smallest_repair")
                        .cloned()
                        .unwrap_or(json!("unknown")),
                    "narrow_rerun": value
                        .pointer("/explanation/narrow_rerun")
                        .cloned()
                        .unwrap_or(json!("unknown")),
                    "broad_rerun": value
                        .pointer("/explanation/broad_rerun")
                        .cloned()
                        .unwrap_or(json!("unknown")),
                    "claim_ceiling": value
                        .pointer("/explanation/claim_ceiling")
                        .cloned()
                        .unwrap_or(json!("unknown"))
                })
            })
        })
        .next()
        .unwrap_or_else(|| json!({"status":"missing"}))
}

fn observed_or_query(value: &Value, observed_key: &str, query_pointer: &str) -> Value {
    value
        .get(observed_key)
        .cloned()
        .or_else(|| value.pointer(query_pointer).cloned())
        .unwrap_or(json!("unknown"))
}

fn explain_matches(value: &Value, candidate: &str, run_id: &str, operation: &str) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(RECEIPT_SCHEMA)
        && value.get("operation").and_then(Value::as_str) == Some("observe.explain-failure")
        && value.get("status").and_then(Value::as_str) == Some("pass")
        && value.get("candidate_digest").and_then(Value::as_str) == Some(candidate)
        && value.get("run_id").and_then(Value::as_str) == Some(run_id)
        && value
            .pointer("/observed_run/operation")
            .and_then(Value::as_str)
            == Some(operation)
}
