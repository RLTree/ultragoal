use crate::cli::control::plane::operation::ControlOperation;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(super) fn attach(
    root: &Path,
    command: &crate::cli::control::plane::ControlCommand,
    receipt_path: &Path,
    value: &mut Value,
    started: Instant,
) -> Result<(), String> {
    let receipt_path = receipt_path.to_string_lossy().into_owned();
    let why = why_failed(value);
    let status = text(value, "status", "fail").to_string();
    let obs = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: command_name(command.operation),
            subcommand: "audit",
            operation: command.operation.id(),
            surface: super::target::surface_id(command.operation),
            law_id: "full-local-observability-stack-integration-non-opaque-failure",
            check_id: check_id(command.operation),
            claim_id: "install_cache_parity",
            artifact_path: "source package and target package surface",
            receipt_path: &receipt_path,
            status: &status,
            failure_class: failure_class(&why),
            why_failed: &why,
            where_failed: where_failed(&status),
            next_repair: next_repair(command.operation, &status),
            claim_impact: claim_impact(command.operation, &status),
            blocked_claims: blocked_claims(value),
            supported_claims: supported_claims(command.operation, &status),
            runtime: Some(runtime(started)),
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
    value["observability"] = obs;
    Ok(())
}

fn runtime(started: Instant) -> crate::cli::observe::telemetry::RuntimeTelemetry {
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
        cache_mode: "package_surface_audit_no_refresh".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_read_serial_package_surface_audit".to_string(),
        repair_anchor_before: "package_surface_audit_start".to_string(),
        repair_anchor_after: "package_surface_observability_emit".to_string(),
    }
}

fn command_name(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::InstallAudit => "ultragoal install",
        ControlOperation::CacheAudit => "ultragoal cache",
        _ => "ultragoal package-surface",
    }
}

pub(super) fn check_id(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::InstallAudit => "install-audit-observability-binding",
        ControlOperation::CacheAudit => "cache-audit-observability-binding",
        _ => "package-surface-audit-observability-binding",
    }
}

fn why_failed(value: &Value) -> String {
    if text(value, "status", "fail") == "pass" {
        return "none".to_string();
    }
    value
        .get("failures")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .unwrap_or("package_surface_audit_failed")
        .to_string()
}

fn failure_class(why: &str) -> &'static str {
    if why == "none" {
        "none"
    } else if why.contains("missing") {
        "missing_surface"
    } else if why.contains("digest") || why.contains("version") {
        "wrong_digest"
    } else {
        "package_surface_audit_failed"
    }
}

fn where_failed(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "package_surface_audit#/failures/0"
    }
}

fn next_repair(operation: ControlOperation, status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else if operation == ControlOperation::CacheAudit {
        "refresh or rebuild the versioned cache package only after source-local proof graph dependencies pass; otherwise keep install/cache claims blocked"
    } else {
        "refresh or rebuild the installed package only after source-local proof graph dependencies pass; otherwise keep install/cache claims blocked"
    }
}

fn claim_impact(operation: ControlOperation, status: &str) -> &'static str {
    if status == "pass" {
        match operation {
            ControlOperation::CacheAudit => {
                "supports_cache_surface_digest_alignment_only_no_readiness_release_completion_update_goal"
            }
            ControlOperation::InstallAudit => {
                "supports_install_surface_digest_alignment_only_no_readiness_release_completion_update_goal"
            }
            _ => {
                "supports_package_surface_digest_alignment_only_no_readiness_release_completion_update_goal"
            }
        }
    } else {
        "blocks_install_cache_parity_readiness_release_completion_update_goal"
    }
}

fn supported_claims(operation: ControlOperation, status: &str) -> Vec<String> {
    if status != "pass" {
        return Vec::new();
    }
    vec![
        match operation {
            ControlOperation::CacheAudit => "cache_surface_digest_alignment",
            ControlOperation::InstallAudit => "install_surface_digest_alignment",
            _ => "package_surface_digest_alignment",
        }
        .to_string(),
    ]
}

fn blocked_claims(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_strings(value.get("blocked_claim_classes"), &mut out);
    collect_strings(value.get("unsupported_claim_classes"), &mut out);
    out
}

fn collect_strings(value: Option<&Value>, out: &mut Vec<String>) {
    if let Some(items) = value.and_then(Value::as_array) {
        for item in items.iter().filter_map(Value::as_str) {
            if !out.iter().any(|existing| existing == item) {
                out.push(item.to_string());
            }
        }
    }
}

fn text<'a>(value: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(fallback)
}

#[cfg(test)]
mod tests;
