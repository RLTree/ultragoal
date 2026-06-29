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
        "retention_bound": "2d",
        "cardinality_guard": "bounded",
        "truncated": rows.len() >= command.row_limit,
        "claim_impact": if status == "pass" { "query_observation_only" } else { "observability_claims_blocked" },
        "result_digest": crate::digest::canonical_json(&Value::Array(rows.clone())),
        "rows": rows,
        "failure": failure.unwrap_or(""),
        "supported_claims": if status == "pass" { json!(["observability_query_observation"]) } else { json!([]) },
        "blocked_claims": json!(["completion", "readiness", "release", "update_goal_eligibility"])
    }))
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
