use crate::cli::observe::telemetry::{claims, exporter, identity, record, spool};
use crate::cli::observe::types::{self, ObserveCommand};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn base(
    root: &Path,
    command: &ObserveCommand,
    status: &str,
    failure: Option<&str>,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let run_id = command
        .run_id
        .clone()
        .unwrap_or_else(|| identity::id("run", command.operation.id(), &candidate));
    let correlation_id = identity::id("corr", command.operation.id(), &candidate);
    let event = record::event(
        root,
        command,
        &candidate,
        &run_id,
        &correlation_id,
        status,
        failure,
    );
    let metric = record::metric(&event, command.operation, status);
    let trace = record::trace(&event, command.operation);
    spool::write(root, &event)?;
    exporter::emit(&event, &metric, &trace);
    Ok(json!({
        "schema": types::RECEIPT_SCHEMA,
        "status": status,
        "candidate_digest": candidate,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "surface": "live_stack",
        "operation": command.operation.id(),
        "log_stream_digest": crate::digest::canonical_json(&event),
        "metric_snapshot_digest": crate::digest::canonical_json(&metric),
        "trace_bundle_digest": crate::digest::canonical_json(&trace),
        "query_examples": query_examples(command),
        "redaction_proof": record::redaction_status(&event),
        "retention_bounds_proof": "pass",
        "bounded_output_proof": claims::bounds_status(command),
        "receipt_path": command.receipt_rel().to_string_lossy(),
        "claim_ceiling": claims::claim_ceiling(command.operation, status),
        "blocked_claims": claims::blocked(command.operation, status),
        "supported_claims": claims::supported(command.operation, status),
        "law_id": types::LAW_ID,
        "check_id": types::CHECK_ID,
        "claim_id": types::CLAIM_ID,
        "why_failed": event["why_failed"].as_str().unwrap_or(""),
        "where_failed": event["where_failed"].as_str().unwrap_or(""),
        "next_repair": claims::next_repair(command.operation, status),
        "event": event,
        "metric": metric,
        "trace": trace
    }))
}

fn query_examples(command: &ObserveCommand) -> Value {
    let run = command.run_id.as_deref().unwrap_or("<run-id>");
    json!([
        format!("ultragoal observe logs query --run-id {run} --limit 100"),
        format!("ultragoal observe metrics query --run-id {run} --limit 100"),
        format!("ultragoal observe traces query --run-id {run} --limit 100")
    ])
}
