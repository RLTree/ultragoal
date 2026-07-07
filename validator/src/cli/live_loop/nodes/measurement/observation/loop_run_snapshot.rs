use super::super::full_command::FullCommandRun;
use super::{TelemetryReconciliation, elapsed_ms};
use crate::cli::live_loop::{LiveLoopCommand, surfaces::LoopValidationSurface};
use serde_json::json;
use std::time::Instant;

pub(crate) fn loop_run_snapshot_pending(
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
) -> TelemetryReconciliation {
    let started = Instant::now();
    let duration_ms = elapsed_ms(started);
    let status = "hot_loop_observation_snapshot_pending";
    let narrow_rerun = format!(
        "target/debug/ultragoal --root . loop measure --node {} --tier {} --cache-mode {}",
        surface.id, command.tier, command.cache_mode
    );
    TelemetryReconciliation {
        status: status.to_string(),
        duration_ms,
        value: json!({
            "status": status,
            "candidate_digest": candidate,
            "node_id": surface.id,
            "surface": surface.surface,
            "command": surface.narrow_rerun,
            "validation_exit_status": actual_work.exit_code,
            "validation_status": if actual_work.status_success { "pass" } else { "fail" },
            "validation_duration_ms": actual_work.duration_ms,
            "stdout_digest": actual_work.stdout_digest,
            "stderr_digest": actual_work.stderr_digest,
            "telemetry_reconciliation_duration_ms": duration_ms,
            "observability_status": "partial",
            "observability_failure_class": status,
            "why_failed": "routine loop run retained the validation result without blocking on per-node live telemetry queries",
            "where_failed": format!("loop.run.{}.observation_snapshot", surface.id),
            "next_repair": format!(
                "run `{narrow_rerun}` for full same-candidate logs metrics traces and explain reconciliation, or repair the loop-level batch telemetry snapshot path"
            ),
            "explain_failure": {
                "value": {
                    "failure_class": status,
                    "where_failed": format!("loop.run.{}.observation_snapshot", surface.id),
                    "why_failed": "validation result is usable and cacheable, but observability proof is intentionally partial until the node is measured or loop-level batch telemetry reconciliation is implemented",
                    "next_repair": format!(
                        "run `{narrow_rerun}` for full same-candidate logs metrics traces and explain reconciliation, or repair the loop-level batch telemetry snapshot path"
                    ),
                    "claim_impact": "validation_result_available_speed_and_observability_claims_withheld"
                }
            },
            "claim_impact": "validation_result_available_speed_and_observability_claims_withheld",
            "claim_ceiling": "source-local hot-loop validation only; no speed observability readiness release completion final-packet or update_goal claim"
        }),
    }
}
