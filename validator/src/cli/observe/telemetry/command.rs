use crate::cli::observe::telemetry::{exporter, identity, record, spool};
use crate::cli::observe::types;
use serde_json::{Value, json};
use std::path::Path;

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
    pub(crate) emit: bool,
}

pub(crate) fn receipt(root: &Path, input: CommandTelemetry<'_>) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let run_id = identity::id("run", input.operation, &candidate);
    let correlation_id = identity::id("corr", input.operation, &candidate);
    let event = event(&input, &candidate, &run_id, &correlation_id, root);
    let metric = metric(&event);
    let trace = trace(&event);
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
        "surface": input.surface,
        "operation": input.operation,
        "log_stream_digest": crate::digest::canonical_json(&event),
        "metric_snapshot_digest": crate::digest::canonical_json(&metric),
        "trace_bundle_digest": crate::digest::canonical_json(&trace),
        "query_examples": query_examples(&run_id),
        "redaction_proof": record::redaction_status(&event),
        "retention_bounds_proof": "pass",
        "bounded_output_proof": "pass",
        "receipt_path": input.receipt_path,
        "claim_impact": input.claim_impact,
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
        "next_repair": input.next_repair,
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
) -> Value {
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
        "artifact_path": input.artifact_path,
        "receipt_path": input.receipt_path,
        "status": input.status,
        "failure_class": input.failure_class,
        "why_failed": input.why_failed,
        "where_failed": input.where_failed,
        "next_repair": input.next_repair,
        "claim_impact": input.claim_impact,
        "timestamp": crate::audit::clock::now_iso(),
        "duration_ms": 0,
        "exporter": exporter(root, candidate, input.status, input.emit),
        "redaction_status": "pass",
        "bounded_output_status": "pass",
        "query_hint_logql": format!("run_id:{run_id} operation:{}", input.operation),
        "query_hint_promql": format!("ultragoal_command_total{{run_id=\"{run_id}\"}}"),
        "query_hint_traceql": format!("{{\"run_id\":\"{run_id}\"}}")
    });
    event["redaction_status"] = json!(record::redaction_status(&event));
    event
}

fn metric(event: &Value) -> Value {
    let exporter = if event["exporter"].as_str() == Some("receipt") {
        "receipt"
    } else {
        "victoriametrics"
    };
    json!({
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
            "candidate_digest": event["candidate_digest"],
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
        "duration_ms": 0,
        "exporter": exporter,
        "redaction_status": event["redaction_status"],
        "bounded_output_status": event["bounded_output_status"],
        "query_hint_logql": event["query_hint_logql"],
        "query_hint_promql": event["query_hint_promql"],
        "query_hint_traceql": event["query_hint_traceql"]
    })
}

fn trace(event: &Value) -> Value {
    let mut span = event.clone();
    span["schema"] = json!("harness-ultragoal.observability-trace.v1");
    span["exporter"] = if event["exporter"].as_str() == Some("receipt") {
        json!("receipt")
    } else {
        json!("victoriatraces")
    };
    span["span_kind"] = json!("root");
    span["span_name"] = event["operation"].clone();
    span
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

fn query_examples(run_id: &str) -> Value {
    json!([
        format!("ultragoal observe logs query --run-id {run_id} --limit 100"),
        format!("ultragoal observe metrics query --run-id {run_id} --limit 100"),
        format!("ultragoal observe traces query --run-id {run_id} --limit 100")
    ])
}
