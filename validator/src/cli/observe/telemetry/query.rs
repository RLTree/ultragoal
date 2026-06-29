use crate::cli::observe::telemetry::identity;
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
    let candidate = crate::package::inventory::package_digest(root)?;
    let run_id = command
        .run_id
        .clone()
        .unwrap_or_else(|| identity::id("query", command.operation.id(), &candidate));
    Ok(json!({
        "schema": types::QUERY_SCHEMA,
        "status": status,
        "candidate_digest": candidate,
        "run_id": run_id,
        "correlation_id": identity::id("corr", command.operation.id(), &candidate),
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
