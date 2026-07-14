use crate::cli::observe::command::{self, ObserveCommand};
use crate::cli::observe::query::observed_failure;
use crate::cli::observe::telemetry::{claims, record};
use serde_json::{Value, json};
use std::path::Path;

mod metric_signal;

pub(super) fn result(
    root: &Path,
    command: &ObserveCommand,
    query_kind: &str,
    query_text: String,
    rows: Vec<Value>,
    status: &str,
    failure: Option<&str>,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    result_for_candidate(
        root, command, query_kind, query_text, rows, status, failure, candidate,
    )
}

pub(super) fn result_for_candidate(
    root: &Path,
    command: &ObserveCommand,
    query_kind: &str,
    query_text: String,
    rows: Vec<Value>,
    status: &str,
    failure: Option<&str>,
    candidate: String,
) -> Result<Value, String> {
    let row_value = Value::Array(rows.clone());
    let row_text = row_value.to_string();
    let redaction_status = record::redaction_status(&row_value);
    let bounded_output_status = claims::bounds_status(command);
    let mut effective_status = status;
    let mut effective_failure = failure;
    if effective_status == "pass" && bounded_output_status != "pass" {
        effective_status = "fail";
        effective_failure = Some("observability_query_bounds_failed");
    }
    if effective_status == "pass" && redaction_status != "pass" {
        effective_status = "fail";
        effective_failure = Some("observability_query_redaction_failed");
    }
    let telemetry = super::receipt::base_for_candidate(
        root,
        command,
        effective_status,
        effective_failure,
        candidate,
    )?;
    let candidate = receipt_text(&telemetry, "candidate_digest")?;
    let run_id = receipt_text(&telemetry, "run_id")?;
    let correlation_id = receipt_text(&telemetry, "correlation_id")?;
    let trace_id = receipt_text(&telemetry, "trace_id")?;
    let failure_class = receipt_text(&telemetry, "failure_class")?;
    let why_failed = receipt_text(&telemetry, "why_failed")?;
    let where_failed = receipt_text(&telemetry, "where_failed")?;
    let next_repair = receipt_text(&telemetry, "next_repair")?;
    let observed_failure = observed_failure(&rows).unwrap_or(Value::Null);
    let observed_record = crate::cli::observe::query::observed_telemetry_record(&rows);
    let observed_failure_class = observed_failure_text(&observed_failure, "failure_class");
    let observed_why_failed = observed_failure_text(&observed_failure, "why_failed");
    let observed_where_failed = observed_failure_text(&observed_failure, "where_failed");
    let observed_next_repair = observed_failure_text(&observed_failure, "next_repair");
    let observed_claim_impact = observed_failure_text(&observed_failure, "claim_impact");
    let metric_summary = metric_signal::summary(query_kind, &rows);
    let mut receipt = json!({
        "schema": command::QUERY_SCHEMA,
        "status": effective_status,
        "candidate_digest": candidate,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "trace_id": trace_id,
        "failure_class": failure_class,
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
        "bounded_output_status": bounded_output_status,
        "redaction_status": redaction_status,
        "claim_impact": if effective_status == "pass" { "query_observation_only" } else { "observability_claims_blocked" },
        "result_digest": crate::digest::canonical_json(&Value::Array(rows.clone())),
        "observed_failure": observed_failure,
        "observed_failure_class": observed_failure_class,
        "observed_why_failed": observed_why_failed,
        "observed_where_failed": observed_where_failed,
        "observed_next_repair": observed_next_repair,
        "observed_claim_impact": observed_claim_impact,
        "rows": rows,
        "failure": effective_failure.unwrap_or(""),
        "supported_claims": if effective_status == "pass" { json!(["observability_query_observation"]) } else { json!([]) },
        "blocked_claims": json!(["completion", "readiness", "release", "update_goal_eligibility"])
    });
    receipt["observed_record"] = observed_record;
    for (key, field) in [
        ("observed_operation", "operation"),
        ("observed_status", "status"),
        ("observed_run_id", "run_id"),
        ("observed_correlation_id", "correlation_id"),
        ("observed_candidate_digest", "candidate_digest"),
    ] {
        receipt[key] = json!(observed_text(&receipt["observed_record"], field));
    }
    receipt["metric_operation"] =
        json!(metric_signal::text(&metric_summary, "operation", "unknown"));
    receipt["metric_status"] = json!(metric_signal::text(&metric_summary, "status", "unknown"));
    receipt["metric_traffic_task_count"] = metric_summary["traffic_task_count"].clone();
    receipt["metric_traffic_count"] = metric_summary["traffic_count"].clone();
    receipt["metric_task_count"] = metric_summary["task_count"].clone();
    receipt["metric_queue_depth"] = metric_summary["queue_depth"].clone();
    receipt["metric_latency_ms"] = metric_summary["latency_ms"].clone();
    receipt["metric_error_count"] = metric_summary["error_count"].clone();
    receipt["metric_latest_sample_unix"] = metric_summary["latest_sample_unix"].clone();
    receipt["metric_event_unix_seconds"] = metric_summary["event_unix_seconds"].clone();
    receipt["metric_failure_class"] = json!(metric_signal::text(
        &metric_summary,
        "failure_class",
        "none"
    ));
    receipt["metric_saturation_status"] = json!(metric_signal::text(
        &metric_summary,
        "saturation_status",
        "unknown"
    ));
    receipt["metric_high_cardinality_labels"] = json!(metric_signal::text(
        &metric_summary,
        "high_cardinality_labels",
        "pass"
    ));
    Ok(receipt)
}

pub(super) fn metric_summary(query_kind: &str, rows: &[Value]) -> Value {
    metric_signal::summary(query_kind, rows)
}

fn observed_failure_text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string()
}

fn observed_text<'a>(value: &'a Value, field: &str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or("none")
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
