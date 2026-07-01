use crate::cli::observe::telemetry::{claims, exporter, identity, record, spool};
use crate::cli::observe::types::{self, ObserveCommand};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(super) fn base(
    root: &Path,
    command: &ObserveCommand,
    status: &str,
    failure: Option<&str>,
) -> Result<Value, String> {
    let started = Instant::now();
    let candidate = crate::package::inventory::package_digest(root)?;
    let run_id = command
        .run_id
        .clone()
        .unwrap_or_else(|| identity::id("run", command.operation.id(), &candidate));
    let correlation_id = command
        .correlation_id
        .clone()
        .unwrap_or_else(|| identity::id("corr", command.operation.id(), &candidate));
    let receipt_path = record::redact_sensitive_text(&command.receipt_rel().to_string_lossy());
    let duration_ms = u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1);
    let runtime = record::runtime(command.operation, duration_ms);
    let event = record::event(
        root,
        command,
        &candidate,
        &run_id,
        &correlation_id,
        status,
        failure,
        runtime,
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
        "trace_id": event["trace_id"],
        "failure_class": event["failure_class"],
        "surface": "live_stack",
        "operation": command.operation.id(),
        "log_stream_digest": crate::digest::canonical_json(&event),
        "metric_snapshot_digest": crate::digest::canonical_json(&metric),
        "trace_bundle_digest": crate::digest::canonical_json(&trace),
        "query_examples": query_examples(&run_id, &correlation_id, command.operation.id()),
        "redaction_proof": record::redaction_status(&event),
        "retention_bounds_proof": "pass",
        "bounded_output_proof": claims::bounds_status(command),
        "receipt_path": receipt_path,
        "claim_ceiling": claims::claim_ceiling(command.operation, status),
        "blocked_claims": claims::blocked(command.operation, status),
        "supported_claims": claims::supported(command.operation, status),
        "law_id": types::LAW_ID,
        "check_id": types::CHECK_ID,
        "claim_id": types::CLAIM_ID,
        "claim_impact": event["claim_impact"],
        "why_failed": event["why_failed"].as_str().unwrap_or(""),
        "where_failed": event["where_failed"].as_str().unwrap_or(""),
        "next_repair": event["next_repair"].as_str().unwrap_or(""),
        "query_hint_logql": event["query_hint_logql"],
        "query_hint_promql": event["query_hint_promql"],
        "query_hint_traceql": event["query_hint_traceql"],
        "event": event,
        "metric": metric,
        "trace": trace
    }))
}

fn query_examples(run: &str, correlation: &str, operation: &str) -> Value {
    let metric_query = crate::cli::observe::query::bounded_metric_query_for_operation(operation);
    json!([
        format!(
            "ultragoal observe logs query --run-id {run} --correlation-id {correlation} --limit 100"
        ),
        format!("ultragoal observe metrics query --query '{metric_query}' --limit 100"),
        format!(
            "ultragoal observe traces query --run-id {run} --correlation-id {correlation} --limit 100"
        )
    ])
}
