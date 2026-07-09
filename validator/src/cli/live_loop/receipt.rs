use super::node_timing_refresh::TimingRefresh;
use crate::cli::live_loop::LiveLoopCommand;
use crate::cli::live_loop::context::AuditContext;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn loop_receipt(
    root: &Path,
    command: &LiveLoopCommand,
    context: AuditContext,
    scheduled: crate::scheduler::Scheduled<Value>,
    current_state: Value,
    first_blocker: Value,
    first_product_blocker: Value,
    first_observability_blocker: Value,
    first_speed_blocker: Value,
    first_control_board_blocker: Value,
    timing_refreshes: Vec<TimingRefresh>,
    status: &str,
    started: Instant,
) -> Result<Value, String> {
    let why_failed =
        (status != "pass").then(|| text(&first_blocker, "why_failed", "loop blocker").to_string());
    let runtime = runtime(command, &scheduled, started);
    let changed_inputs = context.changed_input_summary();
    let package_truth = context.package_truth_summary();
    let receipt_path = command.receipt.to_string_lossy();
    let telemetry_status = telemetry_status(status);
    let telemetry = crate::cli::observe::telemetry::CommandTelemetry {
        command: "ultragoal loop",
        subcommand: "run",
        operation: "loop.run",
        surface: "live_loop",
        law_id: crate::cli::observe::types::LAW_ID,
        check_id: "observability-live-loop-verified-incremental-audit",
        claim_id: "live-loop-hot-repair-feedback",
        artifact_path: "validation_artifacts/current-state.json",
        receipt_path: receipt_path.as_ref(),
        status: telemetry_status,
        failure_class: failure_class(status, &first_blocker),
        why_failed: why_failed.as_deref().unwrap_or("none"),
        where_failed: where_failed(status, &first_blocker),
        next_repair: text(&first_blocker, "next_repair", "none"),
        claim_impact: "source_local_hot_loop_feedback_only_not_full_observability_closure",
        blocked_claims: blocked_claims(),
        supported_claims: vec!["live_loop_hot_repair_feedback".to_string()],
        runtime: Some(runtime),
        emit: true,
    };
    let observability_result = crate::cli::observe::telemetry::command_receipt_for_candidate(
        root,
        telemetry,
        context.candidate_digest.clone(),
    );
    let observability = observability_result?;
    let narrow_rerun = text(&first_blocker, "narrow_rerun", "none").to_string();
    let broad_rerun = text(
        &first_blocker,
        "broad_rerun",
        "source audit once after narrow proof",
    )
    .to_string();
    Ok(json!({
        "schema": "harness-ultragoal.loop-run-receipt.v1",
        "status": status,
        "candidate_digest": context.candidate_digest,
        "tier": command.tier,
        "cache_mode": command.cache_mode,
        "validation_status": validation_status(&first_product_blocker),
        "validation_cache_status": validation_cache_status(status, &first_product_blocker),
        "observability_status": observability_status(&first_observability_blocker),
        "speed_claim_status": speed_claim_status(status, &first_speed_blocker),
        "audit_context": {
            "changed_files_digest": context.changed_files_digest,
            "input_digest": context.input_digest,
            "typed_context": "AuditContext",
            "package_truth": package_truth,
            "changed_inputs": changed_inputs
        },
        "duration_ms": observability["event"]["duration_ms"],
        "worker_count": scheduled.metrics.worker_count,
        "task_count": scheduled.metrics.task_count,
        "queue_depth": scheduled.metrics.queue_depth,
        "critical_path": "current_digest -> AuditContext -> hot_loop_graph -> current_state_downstream_claims",
        "timing_refreshes": timing_refreshes,
        "nodes": scheduled.values,
        "current_state": current_state,
        "first_blocker": first_blocker,
        "first_product_blocker": first_product_blocker,
        "first_observability_blocker": first_observability_blocker,
        "first_speed_blocker": first_speed_blocker,
        "first_control_board_blocker": first_control_board_blocker,
        "narrow_rerun": narrow_rerun,
        "broad_rerun": broad_rerun,
        "forbidden_actions": ["worktrees", "install_cache_refresh", "final_packet_finalization", "readiness_release_completion_claim", "update_goal"],
        "claim_ceiling": "source-local loop proof only",
        "claim_evaluation": claim_evaluation(status, command, &first_blocker),
        "observability": observability
    }))
}

pub(super) fn claim_evaluation(
    status: &str,
    command: &LiveLoopCommand,
    first_blocker: &Value,
) -> Value {
    json!({
        "claim_name": "live-loop hot repair feedback",
        "claim_status": claim_status(status),
        "product_behavior_observed": format!(
            "ultragoal loop run --tier {} --cache-mode {}",
            command.tier, command.cache_mode
        ),
        "proof_surface": if status == "pass" {
            "all high-frequency nodes have executed or verified-cache timing proof and loop receipt"
        } else {
            "no acceleration claim; first blocker names product validation, observability, speed, or downstream claim proof gap"
        },
        "independent_reconciliation_surface": if status == "pass" {
            "same-candidate stdout, receipt, logs, metrics, traces, explain output, and current-state reconciliation"
        } else {
            "blocked until first blocker has same-candidate product behavior and telemetry reconciliation"
        },
        "first_blocker": first_blocker
    })
}

pub(super) fn telemetry_status(status: &str) -> &'static str {
    match status {
        "pass" => "pass",
        "fail" => "fail",
        "partial" => "blocked",
        _ => "blocked",
    }
}

fn claim_status(status: &str) -> &'static str {
    match status {
        "pass" => "supported_source_local",
        "partial" => "withheld_validation_result_available",
        _ => "blocked",
    }
}

pub(super) fn validation_status(first_product_blocker: &Value) -> &'static str {
    if blocker_id(first_product_blocker) == "none" {
        "pass"
    } else {
        "fail"
    }
}

pub(super) fn validation_cache_status(status: &str, first_product_blocker: &Value) -> &'static str {
    if status != "fail" && blocker_id(first_product_blocker) == "none" {
        "reusable"
    } else {
        "not_reusable"
    }
}

pub(super) fn observability_status(first_observability_blocker: &Value) -> &'static str {
    if blocker_id(first_observability_blocker) == "none" {
        "pass"
    } else {
        "partial"
    }
}

pub(super) fn speed_claim_status(status: &str, first_speed_blocker: &Value) -> &'static str {
    if status == "pass" {
        "supported"
    } else if blocker_id(first_speed_blocker) == "none" {
        "withheld"
    } else {
        "failed"
    }
}

fn blocker_id(blocker: &Value) -> &str {
    blocker.get("id").and_then(Value::as_str).unwrap_or("none")
}

fn runtime(
    command: &LiveLoopCommand,
    scheduled: &crate::scheduler::Scheduled<Value>,
    started: Instant,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: scheduled.metrics.worker_count,
        task_count: scheduled.metrics.task_count,
        queue_depth: scheduled.metrics.queue_depth,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: command.cache_mode.clone(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: format!(
            "{};queue_depth={}",
            scheduled.metrics.task_class, scheduled.metrics.queue_depth
        ),
        repair_anchor_before: "current_digest_and_audit_context".to_string(),
        repair_anchor_after: "current_state_and_first_blocker".to_string(),
    }
}

pub(super) fn failure_class<'a>(status: &str, first_blocker: &'a Value) -> &'a str {
    if status == "pass" {
        "none"
    } else {
        text(
            first_blocker,
            "failure_class",
            "observability_live_loop_first_blocker",
        )
    }
}

pub(super) fn where_failed<'a>(status: &str, first_blocker: &'a Value) -> &'a str {
    if status == "pass" {
        "none"
    } else {
        text(first_blocker, "where_failed", "loop.run.first_blocker")
    }
}

fn text<'a>(value: &'a Value, key: &str, default: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn blocked_claims() -> Vec<String> {
    [
        "observability_product_closure",
        "readiness",
        "release",
        "completion",
        "final_packet_correctness",
        "install_cache_parity",
        "app_registry_or_reviewer_exposure",
        "update_goal_eligibility",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}
