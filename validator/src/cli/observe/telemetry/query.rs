use crate::cli::observe::query::observed_failure;
use crate::cli::observe::telemetry::{claims, record};
use crate::cli::observe::types::{self, ObserveCommand};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn result(
    root: &Path,
    command: &ObserveCommand,
    query_kind: &str,
    query_text: String,
    rows: Vec<Value>,
    status: &str,
    failure: Option<&str>,
) -> Result<Value, String> {
    let telemetry = super::receipt::base(root, command, status, failure)?;
    let candidate = receipt_text(&telemetry, "candidate_digest")?;
    let run_id = receipt_text(&telemetry, "run_id")?;
    let correlation_id = receipt_text(&telemetry, "correlation_id")?;
    let trace_id = receipt_text(&telemetry, "trace_id")?;
    let why_failed = receipt_text(&telemetry, "why_failed")?;
    let where_failed = receipt_text(&telemetry, "where_failed")?;
    let next_repair = receipt_text(&telemetry, "next_repair")?;
    let row_value = Value::Array(rows.clone());
    let row_text = row_value.to_string();
    let redaction_status = record::redaction_status(&row_value);
    let observed_failure = observed_failure(&rows).unwrap_or(Value::Null);
    let observed_failure_class = observed_failure_text(&observed_failure, "failure_class");
    let observed_why_failed = observed_failure_text(&observed_failure, "why_failed");
    let observed_where_failed = observed_failure_text(&observed_failure, "where_failed");
    let observed_next_repair = observed_failure_text(&observed_failure, "next_repair");
    let observed_claim_impact = observed_failure_text(&observed_failure, "claim_impact");
    let metric_summary = metric_summary(query_kind, &rows);
    let mut receipt = json!({
        "schema": types::QUERY_SCHEMA,
        "status": status,
        "candidate_digest": candidate,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "trace_id": trace_id,
        "why_failed": why_failed,
        "where_failed": where_failed,
        "next_repair": next_repair,
        "query_kind": query_kind,
        "query": query_text,
        "row_limit": command.row_limit,
        "byte_limit": command.byte_limit,
        "timeout_ms": command.timeout_ms,
        "row_count": rows.len(),
        "byte_count": row_text.len(),
        "retention_bound": "2d",
        "cardinality_guard": "bounded",
        "truncated": rows.len() >= command.row_limit,
        "bounded_output_status": claims::bounds_status(command),
        "redaction_status": redaction_status,
        "claim_impact": if status == "pass" { "query_observation_only" } else { "observability_claims_blocked" },
        "result_digest": crate::digest::canonical_json(&Value::Array(rows.clone())),
        "observed_failure": observed_failure,
        "observed_failure_class": observed_failure_class,
        "observed_why_failed": observed_why_failed,
        "observed_where_failed": observed_where_failed,
        "observed_next_repair": observed_next_repair,
        "observed_claim_impact": observed_claim_impact,
        "rows": rows,
        "failure": failure.unwrap_or(""),
        "supported_claims": if status == "pass" { json!(["observability_query_observation"]) } else { json!([]) },
        "blocked_claims": json!(["completion", "readiness", "release", "update_goal_eligibility"])
    });
    receipt["metric_operation"] = json!(metric_text(&metric_summary, "operation", "unknown"));
    receipt["metric_traffic_task_count"] = metric_summary["traffic_task_count"].clone();
    receipt["metric_latency_ms"] = metric_summary["latency_ms"].clone();
    receipt["metric_error_count"] = metric_summary["error_count"].clone();
    receipt["metric_failure_class"] = json!(metric_text(&metric_summary, "failure_class", "none"));
    receipt["metric_saturation_status"] =
        json!(metric_text(&metric_summary, "saturation_status", "unknown"));
    receipt["metric_high_cardinality_labels"] = json!(metric_text(
        &metric_summary,
        "high_cardinality_labels",
        "pass"
    ));
    Ok(receipt)
}

pub(super) fn metric_summary(query_kind: &str, rows: &[Value]) -> Value {
    if query_kind != "metrics" {
        return json!({});
    }
    let mut operation = "unknown".to_string();
    let mut failure_class = "none".to_string();
    let mut saturation_status = "unknown".to_string();
    let mut high_cardinality_labels = "pass";
    let mut traffic_count = 0_u64;
    let mut task_count = 0_u64;
    let mut latency_ms = 0_u64;
    let mut error_count = 0_u64;
    let mut queue_depth = 0_u64;
    for sample in metric_samples(rows) {
        let labels = sample.get("metric").unwrap_or(&Value::Null);
        if operation == "unknown" {
            operation = metric_label(labels, "operation", "unknown").to_string();
        }
        let sample_failure = metric_label(labels, "failure_class", "none");
        if failure_class == "none" && sample_failure != "none" {
            failure_class = sample_failure.to_string();
        }
        if saturation_status == "unknown" {
            saturation_status = metric_label(labels, "saturation_status", "unknown").to_string();
        }
        if has_high_cardinality_label(labels) {
            high_cardinality_labels = "fail";
        }
        let value = prom_value(&sample);
        match metric_label(labels, "__name__", "ultragoal_command_total") {
            "ultragoal_command_duration_ms" => latency_ms = latency_ms.max(value),
            "ultragoal_command_task_count" => task_count += value,
            "ultragoal_command_queue_depth" => queue_depth = queue_depth.max(value),
            _ => {
                traffic_count += value;
                if metric_label(labels, "status", "pass") != "pass" || sample_failure != "none" {
                    error_count += value;
                }
            }
        }
    }
    json!({
        "operation": operation,
        "traffic_task_count": task_count.max(traffic_count),
        "latency_ms": latency_ms,
        "error_count": error_count,
        "failure_class": failure_class,
        "saturation_status": format!("{saturation_status};queue_depth={queue_depth}"),
        "high_cardinality_labels": high_cardinality_labels
    })
}

fn metric_samples(rows: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for row in rows {
        let Some(body) = row.get("body").and_then(Value::as_str) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(body) else {
            continue;
        };
        if let Some(samples) = value
            .get("data")
            .and_then(|data| data.get("result"))
            .and_then(Value::as_array)
        {
            out.extend(samples.iter().cloned());
        }
    }
    out
}

fn prom_value(sample: &Value) -> u64 {
    sample
        .get("value")
        .and_then(Value::as_array)
        .and_then(|items| items.get(1))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.max(0.0).round() as u64)
        .unwrap_or(0)
}

fn metric_label<'a>(labels: &'a Value, field: &str, default: &'a str) -> &'a str {
    labels.get(field).and_then(Value::as_str).unwrap_or(default)
}

fn has_high_cardinality_label(labels: &Value) -> bool {
    [
        "run_id",
        "correlation_id",
        "trace_id",
        "span_id",
        "candidate_digest",
        "receipt_path",
        "artifact_path",
    ]
    .iter()
    .any(|key| labels.get(*key).is_some())
}

fn metric_text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

fn observed_failure_text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string()
}

fn receipt_text<'a>(telemetry: &'a Value, field: &str) -> Result<&'a str, String> {
    telemetry
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("observability telemetry receipt missing {field}"))
}

#[cfg(test)]
pub(crate) fn receipt_text_for_test<'a>(
    telemetry: &'a Value,
    field: &str,
) -> Result<&'a str, String> {
    receipt_text(telemetry, field)
}
