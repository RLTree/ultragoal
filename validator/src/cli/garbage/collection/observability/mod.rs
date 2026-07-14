use super::GarbageCommand;
use crate::cli::garbage::collection::operation::GarbageOperation;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(super) fn attach(
    root: &Path,
    command: &GarbageCommand,
    receipt: &mut Value,
    started: Instant,
) -> Result<(), String> {
    let status = text(receipt, "status", "fail").to_string();
    let operation = telemetry_operation(command.operation);
    let observability_path = observability_receipt_path(command.operation);
    let artifact_path = command
        .receipt
        .as_ref()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| default_gc_receipt_path(command.operation).to_string());
    let why_failed = why_failed(receipt);
    let obs = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal gc",
            subcommand: cli_subcommand(command.operation),
            operation,
            surface: "workspace_artifact_cache_gc",
            law_id: "workspace-artifact-cache-garbage-collection",
            check_id: "gc-command-observability-binding",
            claim_id: "workspace_gc_observation",
            artifact_path: &artifact_path,
            receipt_path: observability_path,
            status: &status,
            failure_class: failure_class(&why_failed),
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "workspace_gc.command_binding"
            },
            next_repair: next_repair(command.operation, &status),
            claim_impact: claim_impact(&status),
            blocked_claims: blocked_claims(),
            supported_claims: supported_claims(&status),
            runtime: Some(runtime(started, command.operation, &status)),
            emit: true,
        },
    )?;
    let command_receipt_path = crate::output_path::literal_claim_artifact_path(
        root,
        observability_path,
        "GC observability receipt",
    );
    crate::json_boundary::write_json(&command_receipt_path, &obs)?;
    receipt["run_id"] = obs["run_id"].clone();
    receipt["correlation_id"] = obs["correlation_id"].clone();
    receipt["trace_id"] = obs["trace_id"].clone();
    receipt["span_id"] = obs["event"]["span_id"].clone();
    receipt["parent_span_id"] = obs["event"]["parent_span_id"].clone();
    receipt["failure_class"] = obs["failure_class"].clone();
    receipt["why_failed"] = obs["why_failed"].clone();
    receipt["where_failed"] = obs["where_failed"].clone();
    receipt["next_repair"] = obs["next_repair"].clone();
    receipt["claim_impact"] = obs["claim_impact"].clone();
    receipt["receipt_observability_binding"] = json!({
        "artifact_path": artifact_path,
        "command_receipt_path": observability_path,
        "run_id": obs["run_id"],
        "correlation_id": obs["correlation_id"],
        "log_stream_digest": obs["log_stream_digest"],
        "metric_snapshot_digest": obs["metric_snapshot_digest"],
        "trace_bundle_digest": obs["trace_bundle_digest"]
    });
    receipt["observability"] = obs;
    Ok(())
}

fn telemetry_operation(operation: GarbageOperation) -> &'static str {
    match operation {
        GarbageOperation::Plan => "gc_plan",
        GarbageOperation::DryRun => "gc_dry_run",
        GarbageOperation::Apply => "gc_apply",
        GarbageOperation::Verify => "gc_verify",
    }
}

fn cli_subcommand(operation: GarbageOperation) -> &'static str {
    match operation {
        GarbageOperation::Plan => "plan",
        GarbageOperation::DryRun => "dry-run",
        GarbageOperation::Apply => "apply",
        GarbageOperation::Verify => "verify",
    }
}

fn observability_receipt_path(operation: GarbageOperation) -> &'static str {
    match operation {
        GarbageOperation::Plan => "validation_artifacts/observability/gc-plan.json",
        GarbageOperation::DryRun => "validation_artifacts/observability/gc-dry-run.json",
        GarbageOperation::Apply => "validation_artifacts/observability/gc-apply.json",
        GarbageOperation::Verify => "validation_artifacts/observability/gc-verify.json",
    }
}

fn default_gc_receipt_path(operation: GarbageOperation) -> &'static str {
    match operation {
        GarbageOperation::Plan => "validation_artifacts/gc/plan-receipt.json",
        GarbageOperation::DryRun => "validation_artifacts/gc/dry-run-receipt.json",
        GarbageOperation::Apply => "validation_artifacts/gc/apply-receipt.json",
        GarbageOperation::Verify => "validation_artifacts/gc/verify-receipt.json",
    }
}

fn why_failed(receipt: &Value) -> String {
    let failures = receipt
        .get("observation_failures")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    if failures.is_empty() {
        "none".to_string()
    } else {
        failures
    }
}

fn failure_class(why_failed: &str) -> &'static str {
    if why_failed == "none" {
        "none"
    } else if why_failed.contains("plan_digest") {
        "gc_plan_digest_missing"
    } else if why_failed.contains("apply_receipt_digest") {
        "gc_apply_receipt_digest_missing"
    } else {
        "workspace_gc_command_binding_failed"
    }
}

fn next_repair(operation: GarbageOperation, status: &str) -> &'static str {
    if status == "pass" {
        return "none";
    }
    match operation {
        GarbageOperation::Plan => "rerun ultragoal gc plan and preserve the emitted plan digest",
        GarbageOperation::DryRun => "rerun ultragoal gc dry-run with the current gc plan digest",
        GarbageOperation::Apply => "rerun ultragoal gc apply with the current gc plan digest",
        GarbageOperation::Verify => {
            "rerun ultragoal gc verify with the current gc plan digest and apply receipt digest"
        }
    }
}

fn claim_impact(status: &str) -> &'static str {
    if status == "pass" {
        "supports_workspace_gc_observation_only_no_readiness_release_completion_update_goal"
    } else {
        "blocks_workspace_gc_readiness_release_completion_update_goal"
    }
}

fn blocked_claims() -> Vec<String> {
    [
        "readiness",
        "release",
        "completion",
        "final_packet_correctness",
        "update_goal_eligibility",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn supported_claims(status: &str) -> Vec<String> {
    if status == "pass" {
        vec!["workspace_gc_observation".to_string()]
    } else {
        Vec::new()
    }
}

fn runtime(
    started: Instant,
    operation: GarbageOperation,
    status: &str,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: 1,
        task_count: 1,
        queue_depth: usize::from(status != "pass"),
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "workspace_gc_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_write_serial_workspace_gc".to_string(),
        repair_anchor_before: format!("{}_start", telemetry_operation(operation)),
        repair_anchor_after: format!("{}_observability_emit", telemetry_operation(operation)),
    }
}

fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

#[cfg(test)]
mod tests;
