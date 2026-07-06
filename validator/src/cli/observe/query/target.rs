use crate::cli::observe::types::{ObserveCommand, QUERY_SCHEMA, RECEIPT_SCHEMA};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn requested(command: &ObserveCommand) -> Option<String> {
    command
        .run_id
        .as_ref()
        .map(|value| format!("run_id={value}"))
        .or_else(|| {
            command
                .correlation_id
                .as_ref()
                .map(|value| format!("correlation_id={value}"))
        })
        .or_else(|| {
            command
                .check_id
                .as_ref()
                .map(|value| format!("check_id={value}"))
        })
        .or_else(|| {
            command
                .claim_id
                .as_ref()
                .map(|value| format!("claim_id={value}"))
        })
        .or_else(|| {
            command
                .law_id
                .as_ref()
                .map(|value| format!("law_id={value}"))
        })
}

pub(super) fn event(root: &Path, command: &ObserveCommand) -> Option<Value> {
    spool_event(root, |event| matches_target(event, command))
        .or_else(|| receipt_event(root, |event| matches_target(event, command)))
}

fn matches_target(event: &Value, command: &ObserveCommand) -> bool {
    let has_selector = command.run_id.is_some()
        || command.correlation_id.is_some()
        || command.check_id.is_some()
        || command.claim_id.is_some()
        || command.law_id.is_some();
    !is_observation_event(event)
        && matches_optional(event, "run_id", command.run_id.as_deref())
        && matches_optional(event, "correlation_id", command.correlation_id.as_deref())
        && matches_optional(event, "check_id", command.check_id.as_deref())
        && matches_optional(event, "claim_id", command.claim_id.as_deref())
        && matches_optional(event, "law_id", command.law_id.as_deref())
        && has_selector
}

fn matches_optional(event: &Value, field: &str, expected: Option<&str>) -> bool {
    match expected {
        Some(expected) => event.get(field).and_then(Value::as_str) == Some(expected),
        None => true,
    }
}

fn is_observation_event(event: &Value) -> bool {
    let Some(operation) = event.get("operation").and_then(Value::as_str) else {
        return false;
    };
    operation.starts_with("observe.explain-")
        || matches!(
            operation,
            "observe.logs.query" | "observe.metrics.query" | "observe.traces.query"
        )
}

fn spool_event<F>(root: &Path, matches: F) -> Option<Value>
where
    F: Fn(&Value) -> bool,
{
    let path = root.join("validation_artifacts/observability/spool/events.jsonl");
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().rev().find_map(|line| {
        let value: Value = serde_json::from_str(line).ok()?;
        matches(&value).then_some(value)
    })
}

fn receipt_event<F>(root: &Path, matches: F) -> Option<Value>
where
    F: Fn(&Value) -> bool,
{
    let dir = root.join("validation_artifacts/observability");
    let mut paths = Vec::new();
    collect_json_files(&dir, &mut paths);
    paths.sort();
    paths.into_iter().rev().find_map(|path| {
        let value = crate::json_boundary::read_json(&path).ok()?;
        if value.get("schema").and_then(Value::as_str) == Some(QUERY_SCHEMA) {
            return None;
        }
        value
            .get("event")
            .and_then(|event| matches(event).then(|| event.clone()))
            .or_else(|| {
                (value.get("schema").and_then(Value::as_str) == Some(RECEIPT_SCHEMA)
                    && matches(&value))
                .then(|| fallback_event_from_receipt(&value))
            })
    })
}

fn collect_json_files(dir: &Path, paths: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, paths);
        } else if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
}

fn fallback_event_from_receipt(value: &Value) -> Value {
    let operation = value
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    json!({
        "run_id": value.get("run_id").cloned().unwrap_or(Value::Null),
        "correlation_id": value.get("correlation_id").cloned().unwrap_or(Value::Null),
        "candidate_digest": value.get("candidate_digest").cloned().unwrap_or(Value::Null),
        "operation": value.get("operation").cloned().unwrap_or(Value::Null),
        "status": "fail",
        "failure_class": "receipt_without_observability_event",
        "why_failed": format!(
            "observability receipt for {operation} matched the target selector but has no event object"
        ),
        "where_failed": "observe.target.receipt_event_binding",
        "next_repair": format!(
            "rerun {operation} with real event emission, then query logs metrics traces by run/correlation/current digest"
        ),
        "claim_impact": "observability_reconciliation_blocked",
        "observed_receipt_status": value.get("status").cloned().unwrap_or(Value::Null),
        "fallback_only": true,
        "law_id": value.get("law_id").cloned().unwrap_or(Value::Null),
        "check_id": value.get("check_id").cloned().unwrap_or(Value::Null),
        "claim_id": value.get("claim_id").cloned().unwrap_or(Value::Null)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_receipt_discovery_tolerates_missing_observability_directory() {
        let mut paths = Vec::new();
        collect_json_files(Path::new("missing-observability-directory"), &mut paths);
        assert!(paths.is_empty());
    }
}
