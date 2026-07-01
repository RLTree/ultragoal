use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(crate) fn attach(
    root: &Path,
    receipt: &Path,
    value: &mut Value,
    started: Instant,
) -> Result<(), String> {
    let receipt_path = relative_receipt_path(root, receipt)
        .unwrap_or_else(|| "<outside-root-receipt>".to_string());
    let status = text(value, "status", "fail").to_string();
    let why = why_failed(value);
    let obs = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal transaction",
            subcommand: "finalize",
            operation: "transaction_finalize",
            surface: "source_package",
            law_id: "full-local-observability-stack-integration-non-opaque-failure",
            check_id: "transaction-finalize-observability-binding",
            claim_id: "update_goal_eligibility",
            artifact_path: "transactional finalization proof graph",
            receipt_path: &receipt_path,
            status: &status,
            failure_class: failure_class(&why),
            why_failed: &why,
            where_failed: where_failed(&status),
            next_repair: next_repair(&status),
            claim_impact: claim_impact(&status),
            blocked_claims: blocked_claims(value),
            supported_claims: supported_claims(&status),
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
    value["next_repair"] = json!(next_repair(&status));
    value["claim_impact"] = json!(claim_impact(&status));
    value["receipt_observability_binding"] = json!({
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

fn runtime(started: Instant) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: 1,
        task_count: 4,
        queue_depth: 4,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "transaction_finalize_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_write_serial_transaction_finalization".to_string(),
        repair_anchor_before: "transaction_finalize_start".to_string(),
        repair_anchor_after: "transaction_finalize_observability_emit".to_string(),
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
        .unwrap_or_else(|| "transactional_finalization_not_proven".to_string())
}

fn failure_class(why: &str) -> &'static str {
    if why == "none" {
        "none"
    } else if why.contains("coverage") {
        "coverage_blocker"
    } else if why.contains("registry") {
        "registry_blocker"
    } else if why.contains("final_packet") {
        "final_packet_blocker"
    } else {
        "transactional_finalization_blocked"
    }
}

fn where_failed(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "transaction_finalize#/failure/observed_failures/0"
    }
}

fn next_repair(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "repair same-candidate final packet, registry exposure, performance, and exact coverage blockers before rerunning transaction finalize"
    }
}

fn claim_impact(status: &str) -> &'static str {
    if status == "pass" {
        "supports_transactional_finalization_only_no_update_goal_call"
    } else {
        "blocks_transactional_finalization_readiness_release_completion_update_goal"
    }
}

fn supported_claims(status: &str) -> Vec<String> {
    if status == "pass" {
        vec!["transactional_finalization_same_candidate".to_string()]
    } else {
        Vec::new()
    }
}

fn blocked_claims(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(items) = value.get("blocked_claim_classes").and_then(Value::as_array) {
        for item in items.iter().filter_map(Value::as_str) {
            if !out.iter().any(|existing| existing == item) {
                out.push(item.to_string());
            }
        }
    }
    out
}

fn relative_receipt_path(root: &Path, receipt: &Path) -> Option<String> {
    let root_abs = root.canonicalize().ok()?;
    let receipt_abs: PathBuf = if receipt.is_absolute() {
        receipt.to_path_buf()
    } else {
        root_abs.join(receipt)
    };
    receipt_abs
        .strip_prefix(root_abs)
        .ok()
        .map(|rel| rel.to_string_lossy().into_owned())
}

fn text<'a>(value: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(fallback)
}
