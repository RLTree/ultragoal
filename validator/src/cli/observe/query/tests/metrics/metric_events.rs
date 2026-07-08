use serde_json::json;
use std::path::Path;

pub(super) struct RuntimeSignals {
    pub(super) duration_ms: u64,
    pub(super) task_count: u64,
    pub(super) queue_depth: u64,
    pub(super) saturation_status: &'static str,
}

pub(super) fn write_target_event(root: &Path, operation: &str, failure_class: &str) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let event = target_event(operation, "fail", failure_class, candidate, None);
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}

pub(super) fn write_target_event_with_runtime(
    root: &Path,
    operation: &str,
    failure_class: &str,
    runtime: RuntimeSignals,
) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let event = target_event(operation, "fail", failure_class, candidate, Some(runtime));
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}

fn target_event(
    operation: &str,
    status: &str,
    failure_class: &str,
    candidate: String,
    runtime: Option<RuntimeSignals>,
) -> serde_json::Value {
    let mut event = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-query-bound",
        "correlation_id": "corr-query-bound",
        "candidate_digest": candidate,
        "operation": operation,
        "status": status,
        "failure_class": failure_class,
        "why_failed": "target command failed for a specific reason",
        "where_failed": operation,
        "next_repair": "repair the target operation and rerun narrowly",
        "claim_impact": "readiness_release_completion_update_goal_blocked",
        "law_id": crate::cli::observe::types::LAW_ID,
        "check_id": crate::cli::observe::types::CHECK_ID,
        "claim_id": crate::cli::observe::types::CLAIM_ID
    });
    if let Some(runtime) = runtime {
        event["duration_ms"] = json!(runtime.duration_ms);
        event["task_count"] = json!(runtime.task_count);
        event["queue_depth"] = json!(runtime.queue_depth);
        event["saturation_status"] = json!(runtime.saturation_status);
    }
    event
}

pub(super) fn metric_body(operation: &str, failure_class: &str) -> String {
    json!({
        "status": "success",
        "data": {"result": [{
            "metric": {
                "__name__": "ultragoal_command_total",
                "operation": operation,
                "status": "fail",
                "failure_class": failure_class,
                "saturation_status": "serial_command_typed"
            },
            "value": [1, "1"]
        }]}
    })
    .to_string()
}

pub(super) fn metric_body_with_signals(
    operation: &str,
    failure_class: &str,
    runtime: RuntimeSignals,
) -> String {
    let labels = json!({
        "operation": operation,
        "status": "fail",
        "failure_class": failure_class,
        "saturation_status": runtime.saturation_status
    });
    json!({
        "status": "success",
        "data": {"result": [
            {"metric": metric_labels(&labels, "ultragoal_command_total"), "value": [1, "1"]},
            {"metric": metric_labels(&labels, "ultragoal_command_duration_ms"), "value": [1, runtime.duration_ms.to_string()]},
            {"metric": metric_labels(&labels, "ultragoal_command_task_count"), "value": [1, runtime.task_count.to_string()]},
            {"metric": metric_labels(&labels, "ultragoal_command_queue_depth"), "value": [1, runtime.queue_depth.to_string()]}
        ]}
    })
    .to_string()
}

pub(super) fn pass_metric_body(operation: &str) -> String {
    metric_body_with_status(operation, "pass", "none")
}

pub(super) fn metric_body_with_status(
    operation: &str,
    status: &str,
    failure_class: &str,
) -> String {
    json!({
        "status": "success",
        "data": {"result": [{
            "metric": {
                "__name__": "ultragoal_command_total",
                "operation": operation,
                "status": status,
                "failure_class": failure_class,
                "saturation_status": "serial_command_typed"
            },
            "value": [1, "1"]
        }]}
    })
    .to_string()
}

pub(super) fn metric_body_without_total(operation: &str, failure_class: &str) -> String {
    json!({
        "status": "success",
        "data": {"result": [{
            "metric": {
                "__name__": "ultragoal_command_duration_ms",
                "operation": operation,
                "status": "fail",
                "failure_class": failure_class,
                "saturation_status": "serial_command_typed"
            },
            "value": [1, "10"]
        }]}
    })
    .to_string()
}

fn metric_labels(labels: &serde_json::Value, metric_name: &str) -> serde_json::Value {
    let mut out = labels.as_object().cloned().unwrap_or_default();
    out.insert("__name__".to_string(), json!(metric_name));
    serde_json::Value::Object(out)
}
