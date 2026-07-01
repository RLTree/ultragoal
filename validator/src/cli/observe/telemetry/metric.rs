use crate::cli::observe::telemetry::identity;
use serde_json::{Value, json};

pub(crate) fn from_event(event: &Value) -> Value {
    let exporter = if event["exporter"].as_str() == Some("receipt") {
        "receipt"
    } else {
        "victoriametrics"
    };
    let mut metric = json!({
        "schema": "harness-ultragoal.observability-metric.v1",
        "metric_name": "ultragoal_command_total",
        "metric_value": if event["status"].as_str() == Some("pass") { 1 } else { 0 },
        "labels": {
            "command": event["command"],
            "operation": event["operation"],
            "status": event["status"],
            "law_id": event["law_id"],
            "check_id": event["check_id"],
            "claim_id": event["claim_id"],
            "surface": event["surface"],
            "failure_class": event["failure_class"],
            "exporter": exporter
        },
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
