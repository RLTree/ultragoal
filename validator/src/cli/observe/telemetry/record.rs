use crate::cli::observe::telemetry::{RuntimeTelemetry, claims, identity};
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
    runtime: RuntimeTelemetry,
) -> Value {
    let failure = failure.map(redact_sensitive_text);
    let receipt_path = redact_sensitive_text(&command.receipt_rel().to_string_lossy());
    let next_repair = claims::next_repair_for(command.operation, status, failure.as_deref());
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
        "receipt_path": receipt_path,
        "status": status,
        "failure_class": if failure.is_some() { "observability_gate_failure" } else { "none" },
        "why_failed": failure.as_deref().unwrap_or("none"),
        "where_failed": if failure.is_some() { command.operation.id() } else { "none" },
        "next_repair": next_repair,
        "claim_impact": if status == "pass" { "observability_evidence_only" } else { "readiness_release_completion_update_goal_blocked" },
        "timestamp": crate::audit::clock::now_iso(),
        "duration_ms": runtime.duration_ms,
        "exporter": exporter(root, command.operation, candidate, status),
        "redaction_status": "pass",
        "bounded_output_status": claims::bounds_status(command),
        "query_hint_logql": format!("_time:5m operation:{}", command.operation.id()),
        "query_hint_promql": crate::cli::observe::query::bounded_metric_query_for_operation(command.operation.id()),
        "query_hint_traceql": format!("{{operation=\"{}\"}}", command.operation.id())
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
    let mut metric = super::metric::from_event(event);
    if matches!(operation, ObserveOperation::StackHealth) {
        metric["metric_name"] = json!("ultragoal_stack_health_status");
    }
    metric["metric_value"] = json!(if status == "pass" { 1 } else { 0 });
    metric
}

pub(super) fn trace(event: &Value, _operation: ObserveOperation) -> Value {
    super::trace::from_event(event)
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

pub(super) fn redact_sensitive_text(input: &str) -> String {
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

pub(super) fn runtime(operation: ObserveOperation, duration_ms: u64) -> RuntimeTelemetry {
    RuntimeTelemetry {
        duration_ms,
        worker_count: 1,
        task_count: 1,
        queue_depth: 0,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: cache_mode(operation).to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "serial_observe_command_typed".to_string(),
        repair_anchor_before: "observe_command_start".to_string(),
        repair_anchor_after: "observe_telemetry_emit".to_string(),
    }
}

fn cache_mode(operation: ObserveOperation) -> &'static str {
    match operation {
        ObserveOperation::LogsQuery
        | ObserveOperation::MetricsQuery
        | ObserveOperation::TracesQuery => "observe_query_live_backend",
        ObserveOperation::StackHealth | ObserveOperation::StackSmoke => "observe_live_stack_probe",
        _ => "observe_command_no_cache",
    }
}
