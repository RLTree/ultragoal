use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn requested(command: &ObserveCommand) -> Option<String> {
    command
        .run_id
        .as_ref()
        .map(|value| format!("run_id={value}"))
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
    if let Some(run_id) = command.run_id.as_deref() {
        return spool_event(root, |event| {
            event.get("run_id").and_then(Value::as_str) == Some(run_id)
                && !is_observation_event(event)
        })
        .or_else(|| {
            receipt_event(root, |event| {
                event.get("run_id").and_then(Value::as_str) == Some(run_id)
                    && !is_observation_event(event)
            })
        });
    }
    if let Some(check_id) = command.check_id.as_deref() {
        return matching_event(root, "check_id", check_id);
    }
    if let Some(claim_id) = command.claim_id.as_deref() {
        return matching_event(root, "claim_id", claim_id);
    }
    if let Some(law_id) = command.law_id.as_deref() {
        return matching_event(root, "law_id", law_id);
    }
    None
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

fn matching_event(root: &Path, field: &str, expected: &str) -> Option<Value> {
    spool_event(root, |event| {
        event.get(field).and_then(Value::as_str) == Some(expected)
    })
    .or_else(|| {
        receipt_event(root, |event| {
            event.get(field).and_then(Value::as_str) == Some(expected)
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
        if is_query_receipt(&value) {
            return None;
        }
        value
            .get("event")
            .and_then(|event| matches(event).then(|| event.clone()))
            .or_else(|| {
                (is_observability_receipt(&value) && matches(&value))
                    .then(|| fallback_event_from_receipt(&value))
            })
    })
}

fn is_query_receipt(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::QUERY_SCHEMA)
}

fn is_observability_receipt(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::RECEIPT_SCHEMA)
}

fn fallback_event_from_receipt(value: &Value) -> Value {
    json!({
        "run_id": value.get("run_id").cloned().unwrap_or(Value::Null),
        "candidate_digest": value.get("candidate_digest").cloned().unwrap_or(Value::Null),
        "status": value.get("status").cloned().unwrap_or(Value::Null),
        "failure_class": value.get("failure_class").cloned().unwrap_or(Value::Null),
        "why_failed": value.get("why_failed").cloned().unwrap_or(Value::Null),
        "where_failed": value.get("where_failed").cloned().unwrap_or(Value::Null),
        "next_repair": value.get("next_repair").cloned().unwrap_or(Value::Null),
        "claim_impact": value.get("claim_impact").cloned().unwrap_or(Value::Null),
        "law_id": value.get("law_id").cloned().unwrap_or(Value::Null),
        "check_id": value.get("check_id").cloned().unwrap_or(Value::Null),
        "claim_id": value.get("claim_id").cloned().unwrap_or(Value::Null),
        "query_hint_logql": value.get("query_hint_logql").cloned().unwrap_or(Value::Null),
        "query_hint_promql": value.get("query_hint_promql").cloned().unwrap_or(Value::Null),
        "query_hint_traceql": value.get("query_hint_traceql").cloned().unwrap_or(Value::Null)
    })
}
