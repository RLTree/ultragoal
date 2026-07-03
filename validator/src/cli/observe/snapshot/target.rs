use crate::cli::observe::types::{ObserveCommand, RECEIPT_SCHEMA};
use serde_json::{Value, json};
use std::path::Path;

pub(super) struct TargetReceipt {
    pub(super) receipt: Value,
    pub(super) event: Value,
    pub(super) receipt_rel: String,
}

pub(super) fn find_target_receipt(root: &Path, command: &ObserveCommand) -> Option<TargetReceipt> {
    let dir = root.join("validation_artifacts/observability");
    let mut paths = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths.into_iter().rev().find_map(|path| {
        let value = crate::json_boundary::read_json(&path).ok()?;
        let receipt_rel = relative_path_for_root(root, &path);
        target_from_receipt(command, value, receipt_rel)
    })
}

pub(super) fn selectors(command: &ObserveCommand) -> Vec<(&'static str, &str)> {
    [
        ("run_id", command.run_id.as_deref()),
        ("correlation_id", command.correlation_id.as_deref()),
        ("check_id", command.check_id.as_deref()),
        ("claim_id", command.claim_id.as_deref()),
        ("law_id", command.law_id.as_deref()),
    ]
    .into_iter()
    .filter_map(|(field, value)| value.map(|value| (field, value)))
    .collect()
}

pub(super) fn validate_target_event(event: &Value, candidate: &str, failures: &mut Vec<String>) {
    match event.get("candidate_digest").and_then(Value::as_str) {
        Some(found) if found == candidate => {}
        Some(found) => failures.push(format!(
            "target_candidate_digest_mismatch:{found}!={candidate}"
        )),
        None => failures.push("target_candidate_digest_missing".to_string()),
    }
    if event
        .get("fallback_only")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        failures.push("target_receipt_without_event_binding".to_string());
    }
    if event.get("status").and_then(Value::as_str) == Some("fail")
        && ["failure_class", "why_failed", "where_failed", "next_repair"]
            .into_iter()
            .any(|field| empty_or_none(event.get(field).and_then(Value::as_str)))
    {
        failures.push("target_failure_is_not_agent_legible".to_string());
    }
}

pub(super) fn text(event: Option<&Value>, field: &str, default: &str) -> String {
    event
        .and_then(|event| event.get(field))
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

pub(super) fn relative_path_for_root(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn target_from_receipt(
    command: &ObserveCommand,
    receipt: Value,
    receipt_rel: String,
) -> Option<TargetReceipt> {
    if receipt.get("schema").and_then(Value::as_str) != Some(RECEIPT_SCHEMA) {
        return None;
    }
    if let Some(event) = receipt.get("event").cloned() {
        if matches_target(&event, command) {
            return Some(TargetReceipt {
                receipt,
                event,
                receipt_rel,
            });
        }
    }
    if matches_target(&receipt, command) {
        let event = fallback_event_from_receipt(&receipt);
        return Some(TargetReceipt {
            receipt,
            event,
            receipt_rel,
        });
    }
    None
}

fn matches_target(value: &Value, command: &ObserveCommand) -> bool {
    let operation = value
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or_default();
    !operation.starts_with("observe.")
        && selectors(command)
            .into_iter()
            .all(|(field, expected)| value.get(field).and_then(Value::as_str) == Some(expected))
}

fn fallback_event_from_receipt(value: &Value) -> Value {
    let get = |key: &str| value.get(key).cloned().unwrap_or(Value::Null);
    let operation = value
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    json!({
        "run_id": get("run_id"),
        "correlation_id": get("correlation_id"),
        "candidate_digest": get("candidate_digest"),
        "operation": get("operation"),
        "status": "fail",
        "failure_class": "receipt_without_observability_event",
        "why_failed": format!("observability receipt for {operation} matched the target selector but has no event object"),
        "where_failed": "observe.snapshot.target_receipt_event_binding",
        "next_repair": format!("rerun {operation} with real event emission, query logs metrics traces, then rerun observe snapshot"),
        "claim_impact": "observability_reconciliation_blocked",
        "fallback_only": true,
        "law_id": get("law_id"),
        "check_id": get("check_id"),
        "claim_id": get("claim_id")
    })
}

fn empty_or_none(value: Option<&str>) -> bool {
    value.is_none_or(|text| text.trim().is_empty() || text == "none")
}
