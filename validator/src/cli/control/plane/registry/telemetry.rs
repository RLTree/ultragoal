use crate::cli::control::plane::types::ControlOperation;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn supports(operation: ControlOperation) -> bool {
    matches!(
        operation,
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe
    )
}

pub(crate) fn attach(
    root: &Path,
    command: &crate::cli::control::plane::ControlCommand,
    value: &mut Value,
    started: Instant,
) -> Result<(), String> {
    let receipt_path = crate::cli::control::plane::path::expected_receipt_path(command.operation);
    let status = text(value, "status", "fail").to_string();
    let why = why_failed(value);
    let obs = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: command_name(command.operation),
            subcommand: "probe",
            operation: command.operation.id(),
            surface: surface(command.operation),
            law_id: "full-local-observability-stack-integration-non-opaque-failure",
            check_id: check_id(command.operation),
            claim_id: "app_registry_or_reviewer_exposure",
            artifact_path: super::ACTIVE_RECEIPT,
            receipt_path: &receipt_path,
            status: &status,
            failure_class: failure_class(&why),
            why_failed: &why,
            where_failed: where_failed(&status),
            next_repair: next_repair(command.operation, &status),
            claim_impact: claim_impact(command.operation, &status),
            blocked_claims: blocked_claims(value),
            supported_claims: supported_claims(command.operation, &status),
            runtime: Some(runtime(started, command.operation)),
            emit: true,
        },
    )?;
    value["run_id"] = obs["run_id"].clone();
    value["correlation_id"] = obs["correlation_id"].clone();
    value["law_id"] = obs["law_id"].clone();
    value["check_id"] = obs["check_id"].clone();
    value["claim_id"] = obs["claim_id"].clone();
    value["why_failed"] = json!(why);
    value["where_failed"] = json!(where_failed(&status));
    value["next_repair"] = json!(next_repair(command.operation, &status));
    value["claim_impact"] = json!(claim_impact(command.operation, &status));
    value["receipt_observability_binding"] = json!({
        "registry_receipt_path": super::ACTIVE_RECEIPT,
        "command_receipt_path": receipt_path,
        "run_id": obs["run_id"],
        "correlation_id": obs["correlation_id"],
        "log_stream_digest": obs["log_stream_digest"],
        "metric_snapshot_digest": obs["metric_snapshot_digest"],
        "trace_bundle_digest": obs["trace_bundle_digest"]
    });
    value["observability"] = obs;
    Ok(())
}

fn runtime(
    started: Instant,
    operation: ControlOperation,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: 1,
        task_count: 3,
        queue_depth: 3,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "registry_probe_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: saturation(operation).to_string(),
        repair_anchor_before: "registry_probe_start".to_string(),
        repair_anchor_after: "registry_probe_observability_emit".to_string(),
    }
}

fn command_name(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::AppSurfaceProbe => "ultragoal app-surface",
        _ => "ultragoal registry",
    }
}

fn surface(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::AppSurfaceProbe => "codex_desktop_app_surface",
        _ => "codex_desktop_plugin_registry",
    }
}

fn check_id(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::AppSurfaceProbe => "app-surface-probe-observability-binding",
        _ => "registry-probe-observability-binding",
    }
}

fn why_failed(value: &Value) -> String {
    if text(value, "status", "fail") == "pass" {
        return "none".to_string();
    }
    value
        .pointer("/failure/observed_value")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/failure/id").and_then(Value::as_str))
        .unwrap_or("live_registry_reviewer_exposure_not_proven")
        .to_string()
}

fn failure_class(why: &str) -> &'static str {
    if why == "none" {
        "none"
    } else if why.contains("registry") || why.contains("exposure") {
        "external_live_surface_unavailable"
    } else if why.contains("digest") {
        "wrong_digest"
    } else {
        "registry_probe_failed"
    }
}

fn where_failed(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "registry_probe#/evidence_graph/items/registry_exposure/failures/0"
    }
}

fn next_repair(operation: ControlOperation, status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else if operation == ControlOperation::AppSurfaceProbe {
        "provide live same-surface app registry proof or keep app-surface and reviewer claims blocked"
    } else {
        "provide live same-surface registry/reviewer proof or keep registry/reviewer claims blocked"
    }
}

fn claim_impact(operation: ControlOperation, status: &str) -> &'static str {
    if status == "pass" {
        match operation {
            ControlOperation::AppSurfaceProbe => {
                "supports_app_surface_observation_only_no_readiness_release_completion_update_goal"
            }
            _ => {
                "supports_registry_probe_observation_only_no_readiness_release_completion_update_goal"
            }
        }
    } else {
        "blocks_registry_reviewer_readiness_release_completion_update_goal"
    }
}

fn supported_claims(operation: ControlOperation, status: &str) -> Vec<String> {
    if status != "pass" {
        return Vec::new();
    }
    vec![
        match operation {
            ControlOperation::AppSurfaceProbe => "app_surface_same_surface_observation",
            _ => "live_registry_or_reviewer_exposure_same_surface_pass",
        }
        .to_string(),
    ]
}

fn blocked_claims(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_strings(value.get("blocked_claim_classes"), &mut out);
    if let Some(items) = value
        .pointer("/failure/blocked_claim_classes")
        .and_then(Value::as_array)
    {
        for item in items.iter().filter_map(Value::as_str) {
            push_unique(&mut out, item);
        }
    }
    push_unique(&mut out, "app_registry_or_reviewer_exposure");
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

fn saturation(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::AppSurfaceProbe => "external_live_app_surface_probe_serial_typed",
        _ => "external_live_registry_probe_serial_typed_no_capability",
    }
}

fn text<'a>(value: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(fallback)
}
