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
    timing_refreshes: Vec<TimingRefresh>,
    status: &str,
    started: Instant,
) -> Result<Value, String> {
    let why_failed =
        (status != "pass").then(|| text(&first_blocker, "why_failed", "loop blocker").to_string());
    let runtime = runtime(command, &scheduled, started);
    let receipt_path = command.receipt.to_string_lossy();
    let telemetry = crate::cli::observe::telemetry::CommandTelemetry {
        command: "ultragoal loop",
        subcommand: "run",
        operation: "loop.run",
        surface: "observability_live_loop",
        law_id: crate::cli::observe::types::LAW_ID,
        check_id: "observability-live-loop-verified-incremental-audit",
        claim_id: "observability-live-loop-source-local-acceleration",
        artifact_path: "validation_artifacts/current-state.json",
        receipt_path: receipt_path.as_ref(),
        status,
        failure_class: failure_class(status),
        why_failed: why_failed.as_deref().unwrap_or("none"),
        where_failed: where_failed(status),
        next_repair: text(&first_blocker, "next_repair", "none"),
        claim_impact: "source_local_loop_only_not_observability_product_closure",
        blocked_claims: blocked_claims(),
        supported_claims: vec!["observability_live_loop_source_local_increment".to_string()],
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
        "audit_context": {
            "changed_files_digest": context.changed_files_digest,
            "input_digest": context.input_digest,
            "typed_context": "AuditContext"
        },
        "duration_ms": observability["event"]["duration_ms"],
        "worker_count": scheduled.metrics.worker_count,
        "task_count": scheduled.metrics.task_count,
        "queue_depth": scheduled.metrics.queue_depth,
        "critical_path": "current_digest -> AuditContext -> observability_control_board -> current_state",
        "timing_refreshes": timing_refreshes,
        "nodes": scheduled.values,
        "current_state": current_state,
        "first_blocker": first_blocker,
        "narrow_rerun": narrow_rerun,
        "broad_rerun": broad_rerun,
        "forbidden_actions": ["worktrees", "install_cache_refresh", "final_packet_finalization", "readiness_release_completion_claim", "update_goal"],
        "claim_ceiling": "source-local loop proof only",
        "claim_evaluation": claim_evaluation(status, command, &first_blocker),
        "observability": observability
    }))
}

fn claim_evaluation(status: &str, command: &LiveLoopCommand, first_blocker: &Value) -> Value {
    json!({
        "claim_name": "observability live-loop source-local acceleration",
        "claim_status": if status == "pass" { "supported_source_local" } else { "blocked" },
        "product_behavior_observed": format!(
            "ultragoal loop run --tier {} --cache-mode {}",
            command.tier, command.cache_mode
        ),
        "proof_surface": if status == "pass" {
            "all high-frequency nodes have executed or verified-cache timing proof and loop receipt"
        } else {
            "no acceleration claim; first blocker names missing or failed product proof"
        },
        "independent_reconciliation_surface": if status == "pass" {
            "same-candidate stdout, receipt, logs, metrics, traces, explain output, and current-state reconciliation"
        } else {
            "blocked until first blocker has same-candidate product behavior and telemetry reconciliation"
        },
        "first_blocker": first_blocker
    })
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

fn failure_class(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "observability_live_loop_first_blocker"
    }
}

fn where_failed(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "loop.run.current_state.first_blocker"
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

#[cfg(test)]
mod tests {
    use crate::cli::live_loop::{LiveLoopAction, LiveLoopCommand};
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn status_projection_reports_pass_without_failure_fields() {
        assert_eq!(super::failure_class("pass"), "none");
        assert_eq!(super::where_failed("pass"), "none");
        assert_eq!(
            super::failure_class("fail"),
            "observability_live_loop_first_blocker"
        );
        assert_eq!(
            super::where_failed("fail"),
            "loop.run.current_state.first_blocker"
        );
    }

    #[test]
    fn loop_receipt_claim_evaluation_names_pass_and_blocked_surfaces() {
        let command = LiveLoopCommand {
            action: LiveLoopAction::Run,
            tier: "hot".to_string(),
            cache_mode: "verified-local".to_string(),
            jobs: None,
            receipt: PathBuf::from("validation_artifacts/observability/live-loop-run.json"),
            node_id: None,
            measure_all: false,
        };
        let first_blocker = json!({
            "id": "coverage_prove",
            "why_failed": "coverage receipt is stale"
        });

        let pass = super::claim_evaluation("pass", &command, &first_blocker);
        assert_eq!(pass["claim_status"], "supported_source_local");
        assert_eq!(
            pass["product_behavior_observed"],
            "ultragoal loop run --tier hot --cache-mode verified-local"
        );
        assert!(
            pass["proof_surface"]
                .as_str()
                .expect("pass proof surface")
                .contains("executed or verified-cache timing proof")
        );
        assert!(
            pass["independent_reconciliation_surface"]
                .as_str()
                .expect("pass reconciliation")
                .contains("logs, metrics, traces")
        );

        let blocked = super::claim_evaluation("fail", &command, &first_blocker);
        assert_eq!(blocked["claim_status"], "blocked");
        assert!(
            blocked["proof_surface"]
                .as_str()
                .expect("blocked proof surface")
                .contains("no acceleration claim")
        );
        assert_eq!(blocked["first_blocker"]["id"], "coverage_prove");
    }
}
