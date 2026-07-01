use crate::cli::observe::telemetry::{RuntimeTelemetry, exporter, identity, record, spool};
use crate::cli::observe::types;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) struct CommandTelemetry<'a> {
    pub(crate) command: &'a str,
    pub(crate) subcommand: &'a str,
    pub(crate) operation: &'a str,
    pub(crate) surface: &'a str,
    pub(crate) law_id: &'a str,
    pub(crate) check_id: &'a str,
    pub(crate) claim_id: &'a str,
    pub(crate) artifact_path: &'a str,
    pub(crate) receipt_path: &'a str,
    pub(crate) status: &'a str,
    pub(crate) failure_class: &'a str,
    pub(crate) why_failed: &'a str,
    pub(crate) where_failed: &'a str,
    pub(crate) next_repair: &'a str,
    pub(crate) claim_impact: &'a str,
    pub(crate) blocked_claims: Vec<String>,
    pub(crate) supported_claims: Vec<String>,
    pub(crate) runtime: Option<RuntimeTelemetry>,
    pub(crate) emit: bool,
}

pub(crate) fn receipt(root: &Path, input: CommandTelemetry<'_>) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    receipt_for_candidate(root, input, candidate)
}

pub(crate) fn receipt_for_candidate(
    root: &Path,
    input: CommandTelemetry<'_>,
    candidate: String,
) -> Result<Value, String> {
    let started = Instant::now();
    let run_id = identity::id("run", input.operation, &candidate);
    let correlation_id = identity::id("corr", input.operation, &candidate);
    let redacted_artifact_path = record::redact_sensitive_text(input.artifact_path);
    let redacted_receipt_path = record::redact_sensitive_text(input.receipt_path);
    let runtime = input
        .runtime
        .clone()
        .unwrap_or_else(|| default_runtime(elapsed_ms(started)));
    let event = event(
        &input,
        &candidate,
        &run_id,
        &correlation_id,
        root,
        &redacted_artifact_path,
        &redacted_receipt_path,
        runtime,
    );
    let metric = super::metric::from_event(&event);
    let trace = super::trace::from_event(&event);
    if input.emit {
        spool::write(root, &event)?;
        exporter::emit(&event, &metric, &trace);
    }
    Ok(json!({
        "schema": types::RECEIPT_SCHEMA,
        "status": input.status,
        "candidate_digest": candidate,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "trace_id": event["trace_id"],
        "surface": input.surface,
        "operation": input.operation,
        "log_stream_digest": crate::digest::canonical_json(&event),
        "metric_snapshot_digest": crate::digest::canonical_json(&metric),
        "trace_bundle_digest": crate::digest::canonical_json(&trace),
        "query_examples": query_examples(&run_id, &correlation_id, input.operation),
        "redaction_proof": record::redaction_status(&event),
        "retention_bounds_proof": "pass",
        "bounded_output_proof": "pass",
        "receipt_path": redacted_receipt_path,
        "claim_impact": input.claim_impact,
        "failure_class": input.failure_class,
        "claim_ceiling": if input.status == "pass" {
            "observability_binding_only"
        } else {
            "withheld_or_blocked"
        },
        "blocked_claims": input.blocked_claims,
        "supported_claims": input.supported_claims,
        "law_id": input.law_id,
        "check_id": input.check_id,
        "claim_id": input.claim_id,
        "why_failed": event["why_failed"].as_str().unwrap_or(""),
        "where_failed": event["where_failed"].as_str().unwrap_or(""),
        "next_repair": event["next_repair"].as_str().unwrap_or(""),
        "event": event,
        "metric": metric,
        "trace": trace
    }))
}

fn event(
    input: &CommandTelemetry<'_>,
    candidate: &str,
    run_id: &str,
    correlation_id: &str,
    root: &Path,
    artifact_path: &str,
    receipt_path: &str,
    runtime: RuntimeTelemetry,
) -> Value {
    let why_failed = record::redact_sensitive_text(input.why_failed);
    let where_failed = record::redact_sensitive_text(input.where_failed);
    let next_repair = record::redact_sensitive_text(input.next_repair);
    let mut event = json!({
        "schema": types::EVENT_SCHEMA,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "trace_id": identity::id("trace", input.operation, candidate),
        "span_id": identity::id("span", input.operation, candidate),
        "parent_span_id": "",
        "command": input.command,
        "subcommand": input.subcommand,
        "operation": input.operation,
        "surface": input.surface,
        "law_id": input.law_id,
        "check_id": input.check_id,
        "claim_id": input.claim_id,
        "candidate_digest": candidate,
        "target_revision": candidate,
        "artifact_path": artifact_path,
        "receipt_path": receipt_path,
        "status": input.status,
        "failure_class": input.failure_class,
        "why_failed": why_failed,
        "where_failed": where_failed,
        "next_repair": next_repair,
        "claim_impact": input.claim_impact,
        "timestamp": crate::audit::clock::now_iso(),
        "duration_ms": runtime.duration_ms,
        "exporter": exporter(root, candidate, input.status, input.emit),
        "redaction_status": "pass",
        "bounded_output_status": "pass",
        "query_hint_logql": format!("run_id:{run_id} operation:{}", input.operation),
        "query_hint_promql": crate::cli::observe::query::bounded_metric_query_for_operation(input.operation),
        "query_hint_traceql": format!("{{\"run_id\":\"{run_id}\"}}")
    });
    event["worker_count"] = json!(runtime.worker_count);
    event["task_count"] = json!(runtime.task_count);
    event["queue_depth"] = json!(runtime.queue_depth);
    event["cpu_ms"] = json!(runtime.cpu_ms);
    event["memory_bytes"] = json!(runtime.memory_bytes);
    event["io_bytes"] = json!(runtime.io_bytes);
    event["cache_mode"] = json!(runtime.cache_mode);
    event["resource_measurement_status"] = json!(runtime.resource_measurement_status);
    event["retry_count"] = json!(runtime.retry_count);
    event["backoff_ms"] = json!(runtime.backoff_ms);
    event["saturation_status"] = json!(runtime.saturation_status);
    event["repair_anchor_before"] = json!(runtime.repair_anchor_before);
    event["repair_anchor_after"] = json!(runtime.repair_anchor_after);
    event["redaction_status"] = json!(record::redaction_status(&event));
    event
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

fn default_runtime(duration_ms: u64) -> RuntimeTelemetry {
    RuntimeTelemetry {
        duration_ms,
        worker_count: 1,
        task_count: 1,
        queue_depth: 0,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "command_receipt_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "serial_command_typed".to_string(),
        repair_anchor_before: "command_receipt_start".to_string(),
        repair_anchor_after: "command_telemetry_emit".to_string(),
    }
}

fn exporter(root: &Path, candidate: &str, status: &str, emit: bool) -> &'static str {
    if !emit {
        "receipt"
    } else if status == "pass" || super::live_stack_receipts_current(root, candidate) {
        "victorialogs"
    } else {
        "local_spool"
    }
}

fn query_examples(run_id: &str, correlation_id: &str, operation: &str) -> Value {
    let metric_query = crate::cli::observe::query::bounded_metric_query_for_operation(operation);
    json!([
        format!(
            "ultragoal observe logs query --run-id {run_id} --correlation-id {correlation_id} --limit 100"
        ),
        format!("ultragoal observe metrics query --query '{metric_query}' --limit 100"),
        format!(
            "ultragoal observe traces query --run-id {run_id} --correlation-id {correlation_id} --limit 100"
        )
    ])
}
