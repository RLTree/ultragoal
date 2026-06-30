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
    let observed_claim_impact = observed_failure_text(&observed_failure, "claim_impact");
    Ok(json!({
        "schema": types::QUERY_SCHEMA,
        "status": status,
        "candidate_digest": candidate,
        "run_id": run_id,
        "correlation_id": correlation_id,
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
        "observed_claim_impact": observed_claim_impact,
        "rows": rows,
        "failure": failure.unwrap_or(""),
        "supported_claims": if status == "pass" { json!(["observability_query_observation"]) } else { json!([]) },
        "blocked_claims": json!(["completion", "readiness", "release", "update_goal_eligibility"])
    }))
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
