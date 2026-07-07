use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn requested(command: &ObserveCommand) -> Option<String> {
    selected(command).map(|(field, value)| format!("{field}={value}"))
}

pub(super) fn event(root: &Path, command: &ObserveCommand) -> Option<Value> {
    let (field, expected) = selected(command)?;
    let product_event = matching_product_event(root, field, expected);
    if product_event.as_ref().is_some_and(event_failed) {
        return product_event;
    }
    matching_failed_observation_event(root, field, expected).or(product_event)
}

fn selected(command: &ObserveCommand) -> Option<(&'static str, &str)> {
    [
        ("run_id", command.run_id.as_deref()),
        ("check_id", command.check_id.as_deref()),
        ("claim_id", command.claim_id.as_deref()),
        ("law_id", command.law_id.as_deref()),
    ]
    .into_iter()
    .find_map(|(field, value)| value.map(|value| (field, value)))
}

pub(super) fn stale_failure(event: Option<&Value>, candidate: &str) -> Option<String> {
    event.and_then(
        |event| match event.get("candidate_digest").and_then(Value::as_str) {
            Some(event_candidate) if event_candidate == candidate => None,
            Some(event_candidate) => Some(format!(
                "observed telemetry candidate digest mismatch:{event_candidate}!={candidate}"
            )),
            None => Some(format!(
                "observed telemetry candidate digest missing:{}",
                event
                    .get("run_id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            )),
        },
    )
}

pub(super) fn opaque_failure(event: Option<&Value>) -> Option<String> {
    event
        .and_then(|event| {
            event
                .get("fallback_only")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                .then(|| {
                    event
                        .get("why_failed")
                        .and_then(Value::as_str)
                        .unwrap_or("observability receipt matched target without event binding")
                        .to_string()
                })
        })
        .or_else(|| {
            event
                .filter(|event| event.get("status").and_then(Value::as_str) == Some("fail"))
                .filter(|event| event_failure(event).is_none())
                .map(|event| {
                    format!(
                        "observed telemetry failure is opaque:{}",
                        event
                            .get("run_id")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                    )
                })
        })
}

pub(super) fn event_passed(event: &Value) -> bool {
    event.get("status").and_then(Value::as_str) == Some("pass")
}

pub(super) fn event_failure(event: &Value) -> Option<Value> {
    let status = event.get("status").and_then(Value::as_str)?;
    if status == "pass" {
        return None;
    }
    let why = event.get("why_failed").and_then(Value::as_str)?;
    if why.trim().is_empty() || why == "none" {
        return None;
    }
    Some(json!([why]))
}

pub(super) fn promote_fields(receipt: &mut Value, event: &Value) {
    promote_text_fields(
        receipt,
        event,
        &[
            ("observed_status", "status"),
            ("observed_candidate_digest", "candidate_digest"),
            ("observed_artifact_path", "artifact_path"),
            ("observed_receipt_path", "receipt_path"),
            ("observed_failure_class", "failure_class"),
            ("observed_why_failed", "why_failed"),
            ("observed_where_failed", "where_failed"),
            ("observed_next_repair", "next_repair"),
            ("observed_claim_impact", "claim_impact"),
        ],
    );
    for (target, source) in [
        ("observed_duration_ms", "duration_ms"),
        ("observed_worker_count", "worker_count"),
        ("observed_task_count", "task_count"),
        ("observed_queue_depth", "queue_depth"),
    ] {
        if event.get(source).and_then(Value::as_u64).is_some() {
            receipt[target] = event[source].clone();
        }
    }
    for key in [
        "law_id",
        "check_id",
        "claim_id",
        "failure_class",
        "where_failed",
        "next_repair",
        "claim_impact",
        "query_hint_logql",
        "query_hint_promql",
        "query_hint_traceql",
    ] {
        if event
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty() && text != "none")
        {
            receipt[key] = event[key].clone();
        }
    }
}

fn promote_text_fields(receipt: &mut Value, event: &Value, fields: &[(&str, &str)]) {
    for (target, source) in fields {
        if event
            .get(*source)
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty() && text != "none")
        {
            receipt[*target] = event[*source].clone();
        }
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

fn event_failed(event: &Value) -> bool {
    event.get("status").and_then(Value::as_str) == Some("fail")
}

fn matching_product_event(root: &Path, field: &str, expected: &str) -> Option<Value> {
    spool_event(root, |event| {
        event.get(field).and_then(Value::as_str) == Some(expected) && !is_observation_event(event)
    })
    .or_else(|| {
        receipt_event(root, |event| {
            event.get(field).and_then(Value::as_str) == Some(expected)
                && !is_observation_event(event)
        })
    })
}

fn matching_failed_observation_event(root: &Path, field: &str, expected: &str) -> Option<Value> {
    latest_observation_event(root, field, expected)
        .filter(event_failed)
        .or_else(|| super::query::receipt_target::latest_failed_event(root, field, expected))
}

fn latest_observation_event(root: &Path, field: &str, expected: &str) -> Option<Value> {
    spool_event(root, |event| {
        event.get(field).and_then(Value::as_str) == Some(expected) && is_observation_event(event)
    })
    .or_else(|| {
        receipt_event(root, |event| {
            event.get(field).and_then(Value::as_str) == Some(expected)
                && is_observation_event(event)
        })
    })
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
    super::receipt::catalog::observability_json_files(root)
        .into_iter()
        .rev()
        .find_map(|path| {
            let value = crate::json_boundary::read_json(&path).ok()?;
            if is_query_receipt(&value) {
                return None;
            }
            value
                .get("event")
                .and_then(|event| matches(event).then(|| event.clone()))
                .or_else(|| {
                    (is_observability_receipt(&value) && matches(&value))
                        .then(|| super::receipt::event_target::fallback_from_receipt(&value))
                })
        })
}

fn is_query_receipt(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::QUERY_SCHEMA)
}

fn is_observability_receipt(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::RECEIPT_SCHEMA)
}
