use crate::cli::observe::telemetry::identity;
use serde_json::{Value, json};

pub(super) mod export;
#[cfg(test)]
mod summary_tests;

pub(crate) fn from_event(event: &Value) -> Value {
    let exporter = if event["exporter"].as_str() == Some("receipt") {
        "receipt"
    } else {
        "victoriametrics"
    };
    let labels = base_labels(event, exporter);
    let mut metric = json!({
        "schema": "harness-ultragoal.observability-metric.v1",
        "metric_name": "ultragoal_command_total",
        "metric_value": 1,
        "labels": labels,
        "samples": [
            sample("ultragoal_command_total", 1, event, exporter),
            sample("ultragoal_command_duration_ms", number(event, "duration_ms"), event, exporter),
            sample("ultragoal_command_task_count", number(event, "task_count"), event, exporter),
            sample("ultragoal_command_queue_depth", number(event, "queue_depth"), event, exporter),
            sample("ultragoal_command_event_unix_seconds", event_unix(event), event, exporter)
        ],
        "run_id": event["run_id"],
        "correlation_id": event["correlation_id"],
        "trace_id": event["trace_id"],
        "span_id": identity::id(
            "metric",
            event["operation"].as_str().unwrap_or("command"),
            event["candidate_digest"].as_str().unwrap_or("")
        ),
        "parent_span_id": event["span_id"],
        "command": event["command"],
        "subcommand": event["subcommand"],
        "operation": event["operation"],
        "surface": event["surface"],
        "law_id": event["law_id"],
        "check_id": event["check_id"],
        "claim_id": event["claim_id"],
        "candidate_digest": event["candidate_digest"],
        "target_revision": event["target_revision"],
        "artifact_path": event["artifact_path"],
        "receipt_path": event["receipt_path"],
        "status": event["status"],
        "failure_class": event["failure_class"],
        "why_failed": event["why_failed"],
        "where_failed": event["where_failed"],
        "next_repair": event["next_repair"],
        "claim_impact": event["claim_impact"],
        "timestamp": event["timestamp"],
        "duration_ms": event["duration_ms"],
        "exporter": exporter,
        "redaction_status": event["redaction_status"],
        "bounded_output_status": event["bounded_output_status"],
        "query_hint_logql": event["query_hint_logql"],
        "query_hint_promql": event["query_hint_promql"],
        "query_hint_traceql": event["query_hint_traceql"]
    });
    for key in [
        "worker_count",
        "task_count",
        "queue_depth",
        "cpu_ms",
        "memory_bytes",
        "io_bytes",
        "cache_mode",
        "resource_measurement_status",
        "retry_count",
        "backoff_ms",
        "saturation_status",
        "repair_anchor_before",
        "repair_anchor_after",
    ] {
        metric[key] = event[key].clone();
    }
    metric
}

fn sample(name: &str, value: u64, event: &Value, exporter: &str) -> Value {
    json!({
        "metric_name": name,
        "metric_value": value,
        "labels": base_labels(event, exporter),
        "timestamp": event["timestamp"]
    })
}

fn base_labels(event: &Value, exporter: &str) -> Value {
    json!({
        "command": event["command"],
        "operation": event["operation"],
        "status": event["status"],
        "law_id": event["law_id"],
        "check_id": event["check_id"],
        "claim_id": event["claim_id"],
        "surface": event["surface"],
        "failure_class": event["failure_class"],
        "exporter": exporter,
        "saturation_status": event["saturation_status"]
    })
}

fn number(event: &Value, field: &str) -> u64 {
    event.get(field).and_then(Value::as_u64).unwrap_or(0)
}

fn event_unix(event: &Value) -> u64 {
    event
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(crate::audit::clock::parse_iso_seconds)
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(0)
}
