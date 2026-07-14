use crate::cli::observe::command::{ObserveCommand, ObserveOperation};
use crate::cli::observe::telemetry;
use serde_json::Value;
use std::path::Path;

use super::{
    bounded_metric_query_for_operation_status, bounded_success_metric_query_for_operation, target,
};

const METRIC_QUERY_WINDOW_SECONDS: i64 = 300;

pub(super) fn target_query(root: &Path, command: &ObserveCommand) -> Option<String> {
    if command.operation != ObserveOperation::MetricsQuery || command.query.is_some() {
        return None;
    }
    let event = target::event(root, command)?;
    let operation = event.get("operation").and_then(Value::as_str)?;
    match event.get("status").and_then(Value::as_str) {
        Some("pass") => Some(bounded_success_metric_query_for_operation(operation)),
        Some(status) => Some(bounded_metric_query_for_operation_status(operation, status)),
        None => None,
    }
}

pub(super) fn reconciliation_failure(
    root: &Path,
    command: &ObserveCommand,
    rows: &[Value],
    candidate: &str,
) -> Option<String> {
    let observed = target::event(root, command);
    if observed.is_none() && (command.run_id.is_some() || command.correlation_id.is_some()) {
        let requested = target::requested(command)
            .expect("missing target branch requires run or correlation selector");
        return Some(format!(
            "observability_metric_target_unavailable:{}",
            requested
        ));
    }
    let event = observed.as_ref()?;
    if let Some(failure) = candidate_failure(event, candidate) {
        return Some(failure);
    }
    let target_operation = event.get("operation").and_then(Value::as_str).unwrap_or("");
    let summary = telemetry::metric_summary(rows);
    let metric_operation = text_field(&summary, "operation", "unknown");
    if target_operation.is_empty() || metric_operation == "unknown" {
        return Some(format!(
            "observability_metric_missing_for_target:{}",
            target_operation_or_unknown(target_operation)
        ));
    }
    if metric_operation != target_operation {
        return Some(format!(
            "observability_metric_operation_mismatch:{metric_operation}!={target_operation}"
        ));
    }
    if let Some(failure) = status_mismatch(event, &summary) {
        return Some(failure);
    }
    if let Some(failure) = failure_class_mismatch(event, &summary) {
        return Some(failure);
    }
    if let Some(failure) = freshness_mismatch(event, &summary) {
        return Some(failure);
    }
    target_signal_mismatch(event, &summary)
}

fn candidate_failure(event: &Value, candidate: &str) -> Option<String> {
    match event.get("candidate_digest").and_then(Value::as_str) {
        Some(value) if value == candidate => None,
        Some(value) => Some(format!(
            "observability_metric_candidate_mismatch:{value}!={candidate}"
        )),
        None => Some("observability_metric_candidate_missing".to_string()),
    }
}

fn failure_class_mismatch(event: &Value, summary: &Value) -> Option<String> {
    let target_failure = event
        .get("failure_class")
        .and_then(Value::as_str)
        .unwrap_or("none");
    if target_failure == "none" {
        let metric_failure = text_field(summary, "failure_class", "none");
        let error_count = summary
            .get("error_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        return (metric_failure != "none" || error_count > 0).then(|| {
            format!(
                "observability_metric_pass_target_has_error_signal:{metric_failure}:error_count={error_count}"
            )
        });
    }
    let metric_failure = text_field(summary, "failure_class", "none");
    if metric_failure != target_failure {
        return Some(format!(
            "observability_metric_failure_mismatch:{metric_failure}!={target_failure}"
        ));
    }
    (summary
        .get("error_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0)
        .then(|| format!("observability_metric_error_count_missing:{target_failure}"))
}

fn status_mismatch(event: &Value, summary: &Value) -> Option<String> {
    let target_status = event.get("status").and_then(Value::as_str).unwrap_or("");
    let metric_status = text_field(summary, "status", "unknown");
    (target_status.is_empty() || metric_status == "unknown" || metric_status != target_status).then(
        || {
            format!(
                "observability_metric_status_mismatch:{metric_status}!={}",
                target_status_or_unknown(target_status)
            )
        },
    )
}

fn target_signal_mismatch(event: &Value, summary: &Value) -> Option<String> {
    underreported_signal(event, summary, "duration_ms", "latency_ms")
        .or_else(|| underreported_signal(event, summary, "task_count", "task_count"))
        .or_else(|| underreported_signal(event, summary, "queue_depth", "queue_depth"))
}

fn freshness_mismatch(event: &Value, summary: &Value) -> Option<String> {
    let target_timestamp = event.get("timestamp").and_then(Value::as_str)?;
    let target_unix = crate::audit::clock::parse_iso_seconds(target_timestamp)?;
    let metric_event_unix = summary.get("event_unix_seconds").and_then(Value::as_u64)?;
    let target_unix_u64 = u64::try_from(target_unix).ok()?;
    if metric_event_unix < target_unix_u64 {
        return Some(format!(
            "observability_metric_event_time_stale:metric_event_unix={} target_event_unix={}",
            metric_event_unix, target_unix_u64
        ));
    }
    let latest_sample = summary.get("latest_sample_unix").and_then(Value::as_i64)?;
    if latest_sample == 0 || latest_sample <= target_unix + METRIC_QUERY_WINDOW_SECONDS {
        return None;
    }
    Some(format!(
        "observability_metric_target_outside_query_window:target_timestamp={} latest_metric_timestamp={} window_seconds={}",
        target_timestamp,
        crate::audit::clock::unix_to_iso(latest_sample),
        METRIC_QUERY_WINDOW_SECONDS
    ))
}

fn underreported_signal(
    event: &Value,
    summary: &Value,
    target_field: &str,
    metric_field: &str,
) -> Option<String> {
    let target = event.get(target_field).and_then(Value::as_u64)?;
    let metric = summary.get(metric_field).and_then(Value::as_u64)?;
    (metric < target).then(|| {
        format!(
            "observability_metric_run_reconciliation_mismatch:{target_field} metric={metric} target={target}"
        )
    })
}

fn text_field<'a>(value: &'a Value, field: &str, fallback: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(fallback)
}

fn target_operation_or_unknown(operation: &str) -> &str {
    if operation.is_empty() {
        "unknown"
    } else {
        operation
    }
}

fn target_status_or_unknown(status: &str) -> &str {
    if status.is_empty() { "unknown" } else { status }
}
