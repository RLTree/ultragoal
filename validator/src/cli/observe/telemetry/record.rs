use crate::cli::observe::telemetry::{claims, identity};
use crate::cli::observe::types::{self, ObserveCommand, ObserveOperation};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn event(
    root: &Path,
    command: &ObserveCommand,
    candidate: &str,
    run_id: &str,
    correlation_id: &str,
    status: &str,
    failure: Option<&str>,
) -> Value {
    let failure = failure.map(redact_sensitive_text);
    let mut event = json!({
        "schema": types::EVENT_SCHEMA,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "trace_id": identity::id("trace", command.operation.id(), candidate),
        "span_id": identity::id("span", command.operation.id(), candidate),
        "parent_span_id": "",
        "command": "ultragoal observe",
        "subcommand": command.operation.subcommand(),
        "operation": command.operation.id(),
        "surface": "live_stack",
        "law_id": types::LAW_ID,
        "check_id": command.check_id.as_deref().unwrap_or(types::CHECK_ID),
        "claim_id": command.claim_id.as_deref().unwrap_or(types::CLAIM_ID),
        "candidate_digest": candidate,
        "target_revision": candidate,
        "artifact_path": "dev/observability",
        "receipt_path": command.receipt_rel().to_string_lossy(),
        "status": status,
        "failure_class": if failure.is_some() { "observability_gate_failure" } else { "none" },
        "why_failed": failure.as_deref().unwrap_or("none"),
        "where_failed": if failure.is_some() { command.operation.id() } else { "none" },
        "next_repair": claims::next_repair(command.operation, status),
        "claim_impact": if status == "pass" { "observability_evidence_only" } else { "readiness_release_completion_update_goal_blocked" },
        "timestamp": crate::audit::clock::now_iso(),
        "duration_ms": 0,
        "exporter": exporter(root, command.operation, candidate, status),
        "redaction_status": "pass",
        "bounded_output_status": claims::bounds_status(command),
        "query_hint_logql": format!("_time:5m operation:{}", command.operation.id()),
        "query_hint_promql": format!("ultragoal_command_total{{operation=\"{}\"}}", command.operation.id()),
        "query_hint_traceql": format!("{{operation=\"{}\"}}", command.operation.id())
    });
    event["redaction_status"] = json!(redaction_status(&event));
    event
}

fn exporter(
    root: &Path,
    operation: ObserveOperation,
    candidate: &str,
    status: &str,
) -> &'static str {
    if status == "pass"
        && matches!(
            operation,
            ObserveOperation::StackHealth | ObserveOperation::StackSmoke
        )
    {
        "victorialogs"
    } else if super::live_stack_receipts_current(root, candidate) {
        "victorialogs"
    } else {
        "local_spool"
    }
}

pub(super) fn metric(event: &Value, operation: ObserveOperation, status: &str) -> Value {
    let metric_name = if matches!(operation, ObserveOperation::StackHealth) {
        "ultragoal_stack_health_status"
    } else {
        "ultragoal_command_total"
    };
    json!({
        "schema": "harness-ultragoal.observability-metric.v1",
        "metric_name": metric_name,
        "metric_value": if status == "pass" { 1 } else { 0 },
        "labels": labels(event, operation, status),
        "run_id": event["run_id"],
        "correlation_id": event["correlation_id"],
        "trace_id": event["trace_id"],
        "span_id": identity::id("metric", operation.id(), event["candidate_digest"].as_str().unwrap_or("")),
        "parent_span_id": event["span_id"],
        "command": "ultragoal observe",
        "subcommand": operation.subcommand(),
        "operation": operation.id(),
        "surface": "live_stack",
        "law_id": types::LAW_ID,
        "check_id": types::CHECK_ID,
        "claim_id": types::CLAIM_ID,
        "candidate_digest": event["candidate_digest"],
        "target_revision": event["target_revision"],
        "artifact_path": event["artifact_path"],
        "receipt_path": event["receipt_path"],
        "status": status,
        "failure_class": event["failure_class"],
        "why_failed": event["why_failed"],
        "where_failed": event["where_failed"],
        "next_repair": event["next_repair"],
        "claim_impact": event["claim_impact"],
        "timestamp": event["timestamp"],
        "duration_ms": 0,
        "exporter": "victoriametrics",
        "redaction_status": event["redaction_status"],
        "bounded_output_status": event["bounded_output_status"],
        "query_hint_logql": event["query_hint_logql"],
        "query_hint_promql": event["query_hint_promql"],
        "query_hint_traceql": event["query_hint_traceql"]
    })
}

pub(super) fn trace(event: &Value, operation: ObserveOperation) -> Value {
    let mut span = event.clone();
    span["schema"] = json!("harness-ultragoal.observability-trace.v1");
    span["exporter"] = json!("victoriatraces");
    span["span_kind"] = json!("root");
    span["span_name"] = json!(operation.id());
    span
}

pub(super) fn redaction_status(event: &Value) -> &'static str {
    let lower = event.to_string().to_ascii_lowercase();
    if sensitive_markers()
        .iter()
        .any(|needle| lower.contains(needle))
    {
        "fail"
    } else {
        "pass"
    }
}

fn sensitive_markers() -> Vec<String> {
    let home = private_home_marker();
    let temp = private_tmp_marker();
    [
        "authorization".to_string(),
        "api_key".to_string(),
        "token=".to_string(),
        "cookie".to_string(),
        "database_url".to_string(),
        home.to_ascii_lowercase(),
        format!("file://{}", home.to_ascii_lowercase()),
        format!("unix://{}", home.to_ascii_lowercase()),
        temp.to_ascii_lowercase(),
        format!("file://{}", temp.to_ascii_lowercase()),
        format!("unix://{}", temp.to_ascii_lowercase()),
    ]
    .into()
}

fn redact_sensitive_text(input: &str) -> String {
    let home = redact_path_marker(input, private_home_marker(), "[redacted-home-path]");
    let private_tmp =
        redact_path_marker(&home, private_tmp_marker(), "[redacted-private-tmp-path]");
    private_tmp
        .replace("file://[redacted-home-path]", "[redacted-home-path]")
        .replace("unix://[redacted-home-path]", "[redacted-home-path]")
        .replace(
            "file://[redacted-private-tmp-path]",
            "[redacted-private-tmp-path]",
        )
}

fn private_home_marker() -> &'static str {
    concat!("/", "Users/")
}

fn private_tmp_marker() -> &'static str {
    concat!("/", "private", "/tmp/")
}

fn redact_path_marker(body: &str, marker: &str, replacement: &str) -> String {
    let mut output = String::with_capacity(body.len());
    let mut index = 0;
    while let Some(offset) = body[index..].find(marker) {
        let start = index + offset;
        output.push_str(&body[index..start]);
        let end = body[start..]
            .char_indices()
            .find_map(|(idx, ch)| path_delimiter(ch).then_some(start + idx))
            .unwrap_or(body.len());
        output.push_str(replacement);
        index = end;
    }
    output.push_str(&body[index..]);
    output
}

fn path_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | '}' | ']' | ')')
}

#[cfg(test)]
pub(crate) fn redacted_failure_for_test(input: &str) -> String {
    redact_sensitive_text(input)
}

fn labels(event: &Value, operation: ObserveOperation, status: &str) -> Value {
    json!({
        "command": "observe",
        "operation": operation.id(),
        "status": status,
        "law_id": types::LAW_ID,
        "check_id": types::CHECK_ID,
        "claim_id": types::CLAIM_ID,
        "surface": "live_stack",
        "failure_class": event["failure_class"].as_str().unwrap_or("none"),
        "candidate_digest": event["candidate_digest"].as_str().unwrap_or(""),
        "exporter": event["exporter"].as_str().unwrap_or("")
    })
}
