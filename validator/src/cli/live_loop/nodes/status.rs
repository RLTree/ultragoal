use super::super::surfaces::LoopValidationSurface;

pub(crate) struct MeasurementState {
    pub(crate) status: &'static str,
    pub(crate) failure_class: &'static str,
    pub(crate) why_failed: String,
    pub(crate) where_failed: String,
    pub(crate) next_repair: String,
    pub(crate) baseline_state: &'static str,
    pub(crate) speedup_state: &'static str,
    pub(crate) baseline_duration_ms: Option<u64>,
    pub(crate) speedup_ratio: Option<u64>,
    pub(crate) claim_impact: String,
}

pub(crate) fn measurement_state(
    surface: LoopValidationSurface,
    duration_ms: u64,
    baseline_ms: Option<u64>,
) -> MeasurementState {
    if !surface.high_frequency {
        return MeasurementState {
            status: "observed",
            failure_class: "none",
            why_failed: "diagnostic context node observed; no speed or closure claim is supported"
                .to_string(),
            where_failed: "none".to_string(),
            next_repair: "none".to_string(),
            baseline_state: "not_required_for_context_or_control_node",
            speedup_state: "not_required_for_context_or_control_node",
            baseline_duration_ms: baseline_ms,
            speedup_ratio: baseline_ms.map(|value| value / duration_ms.max(1)),
            claim_impact:
                "observation_only_no_speed_readiness_release_completion_or_update_goal_claim"
                    .to_string(),
        };
    }
    if let Some(baseline_duration_ms) = baseline_ms {
        let speedup_ratio = baseline_duration_ms / duration_ms.max(1);
        if speedup_ratio >= 20 {
            return MeasurementState {
                status: "pass",
                failure_class: "none",
                why_failed: "none".to_string(),
                where_failed: "none".to_string(),
                next_repair: "none".to_string(),
                baseline_state: "current_full_command_baseline_observed",
                speedup_state: "verified_local_20x_proof_observed",
                baseline_duration_ms: Some(baseline_duration_ms),
                speedup_ratio: Some(speedup_ratio),
                claim_impact: "supports_source_local_live_loop_node_measurement_only".to_string(),
            };
        }
        return MeasurementState {
            status: "blocked",
            failure_class: "live_loop_speedup_target_missed",
            why_failed: "high-frequency live-loop node has baseline timing but does not meet the verified-local 20x speed target".to_string(),
            where_failed: format!("loop.run.{}.speedup", surface.id),
            next_repair: format!(
                "split, cache, daemonize, or re-architect `{}` until verified-local timing is at least 20x faster than baseline `{}`",
                surface.id, surface.canonical_full_command
            ),
            baseline_state: "current_full_command_baseline_observed",
            speedup_state: "verified_local_20x_proof_failed",
            baseline_duration_ms: Some(baseline_duration_ms),
            speedup_ratio: Some(speedup_ratio),
            claim_impact: "blocks_live_loop_routine_repair_until_current_timing_proof".to_string(),
        };
    }
    MeasurementState {
        status: "blocked",
        failure_class: "live_loop_high_frequency_measurement_missing",
        why_failed: "high-frequency live-loop node lacks current full-command baseline and verified-local 20x timing proof".to_string(),
        where_failed: format!("loop.run.{}.measurement", surface.id),
        next_repair: format!(
            "measure canonical baseline `{}` and verified-local node timing for `{}`, bind both to current inputs, then rerun `target/debug/ultragoal --root . loop run --tier hot --cache-mode verified-local --jobs auto`",
            surface.canonical_full_command, surface.id
        ),
        baseline_state: "missing_current_full_command_baseline",
        speedup_state: "missing_verified_local_20x_proof",
        baseline_duration_ms: None,
        speedup_ratio: None,
        claim_impact: "blocks_live_loop_routine_repair_until_current_timing_proof".to_string(),
    }
}
