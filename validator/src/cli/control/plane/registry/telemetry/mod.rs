use crate::cli::control::plane::types::ControlOperation;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

mod catalog;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod tests;

pub(crate) fn attach(
    root: &Path,
    command: &crate::cli::control::plane::ControlCommand,
    value: &mut Value,
    started: Instant,
) -> Result<(), String> {
    let receipt_path = crate::cli::control::plane::path::expected_receipt_path(command.operation);
    let status = text(value, "status", "fail").to_string();
    let why = why_failed(value);
    let (command_name, subcommand) = catalog::command_parts(command.operation);
    let where_failed = catalog::where_failed(command.operation, &status);
    let next_repair = catalog::next_repair(command.operation, &status);
    let claim_impact = catalog::claim_impact(command.operation, &status);
    let artifact_path = artifact_path(command.operation, &receipt_path);
    let obs = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: command_name,
            subcommand,
            operation: command.operation.id(),
            surface: catalog::surface(command.operation),
            law_id: "full-local-observability-stack-integration-non-opaque-failure",
            check_id: catalog::check_id(command.operation),
            claim_id: catalog::claim_id(command.operation),
            artifact_path: &artifact_path,
            receipt_path: &receipt_path,
            status: &status,
            failure_class: catalog::failure_class(&why),
            why_failed: &why,
            where_failed: &where_failed,
            next_repair: &next_repair,
            claim_impact: &claim_impact,
            blocked_claims: blocked_claims(value),
            supported_claims: catalog::supported_claims(command.operation, &status),
            runtime: Some(runtime(started, command.operation, value)),
            emit: true,
        },
    )?;
    value["run_id"] = obs["run_id"].clone();
    value["correlation_id"] = obs["correlation_id"].clone();
    value["trace_id"] = obs["event"]["trace_id"].clone();
    value["span_id"] = obs["event"]["span_id"].clone();
    value["parent_span_id"] = obs["event"]["parent_span_id"].clone();
    value["law_id"] = obs["law_id"].clone();
    value["check_id"] = obs["check_id"].clone();
    value["claim_id"] = obs["claim_id"].clone();
    value["failure_class"] = obs["failure_class"].clone();
    value["why_failed"] = json!(why);
    value["where_failed"] = json!(where_failed);
    value["next_repair"] = json!(next_repair);
    value["claim_impact"] = json!(claim_impact);
    value["duration_ms"] = obs["event"]["duration_ms"].clone();
    value["worker_count"] = obs["event"]["worker_count"].clone();
    value["task_count"] = obs["event"]["task_count"].clone();
    value["queue_depth"] = obs["event"]["queue_depth"].clone();
    value["cache_mode"] = obs["event"]["cache_mode"].clone();
    value["saturation_status"] = obs["event"]["saturation_status"].clone();
    let mut binding = json!({
        "artifact_path": artifact_path,
        "command_receipt_path": receipt_path,
        "run_id": obs["run_id"],
        "correlation_id": obs["correlation_id"],
        "log_stream_digest": obs["log_stream_digest"],
        "metric_snapshot_digest": obs["metric_snapshot_digest"],
        "trace_bundle_digest": obs["trace_bundle_digest"]
    });
    if catalog::registry_surface(command.operation) {
        binding["registry_receipt_path"] = json!(super::ACTIVE_RECEIPT);
    }
    value["receipt_observability_binding"] = binding;
    value["observability"] = obs;
    Ok(())
}

fn runtime(
    started: Instant,
    operation: ControlOperation,
    value: &Value,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    let task_count = task_count(operation, value);
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: 1,
        task_count,
        queue_depth: task_count,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: catalog::cache_mode(operation).to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: catalog::saturation(operation).to_string(),
        repair_anchor_before: format!("{}_start", operation.id()),
        repair_anchor_after: format!("{}_observability_emit", operation.id()),
    }
}

fn artifact_path(operation: ControlOperation, receipt_path: &str) -> String {
    if catalog::registry_surface(operation) {
        super::ACTIVE_RECEIPT.to_string()
    } else {
        receipt_path.to_string()
    }
}

fn why_failed(value: &Value) -> String {
    if text(value, "status", "fail") == "pass" {
        return "none".to_string();
    }
    value
        .pointer("/failure/observed_failures")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .filter(|text| !text.is_empty())
        .or_else(|| {
            value
                .pointer("/failure/observed_value")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            value
                .pointer("/failure/id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "control_plane_evidence_not_proven".to_string())
}

fn blocked_claims(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_strings(value.get("blocked_claim_classes"), &mut out);
    if value
        .pointer("/failure/observed_value")
        .and_then(Value::as_str)
        .is_some_and(|text| text.contains("update_goal"))
    {
        push_unique(&mut out, "update_goal_eligibility");
    }
    if let Some(items) = value
        .pointer("/failure/blocked_claim_classes")
        .and_then(Value::as_array)
    {
        for item in items.iter().filter_map(Value::as_str) {
            push_unique(&mut out, item);
        }
    }
    if out.is_empty() && text(value, "status", "fail") != "pass" {
        for claim in [
            "readiness",
            "release",
            "completion",
            "update_goal_eligibility",
        ] {
            push_unique(&mut out, claim);
        }
    }
    out
}

fn collect_strings(value: Option<&Value>, out: &mut Vec<String>) {
    if let Some(items) = value.and_then(Value::as_array) {
        for item in items.iter().filter_map(Value::as_str) {
            push_unique(out, item);
        }
    }
}

fn push_unique(out: &mut Vec<String>, item: &str) {
    if !out.iter().any(|existing| existing == item) {
        out.push(item.to_string());
    }
}

fn task_count(operation: ControlOperation, value: &Value) -> usize {
    if catalog::registry_surface(operation) {
        return 3;
    }
    if let Some(items) = value
        .pointer("/failure/observed_failures")
        .and_then(Value::as_array)
    {
        return items.len().max(1);
    }
    value
        .pointer("/failure/observed_value")
        .and_then(Value::as_str)
        .map(|text| {
            text.split(" | ")
                .filter(|part| !part.trim().is_empty())
                .count()
        })
        .unwrap_or(1)
        .max(1)
}

fn text<'a>(value: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(fallback)
}
