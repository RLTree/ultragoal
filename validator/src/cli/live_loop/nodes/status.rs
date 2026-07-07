use super::super::surfaces::{BOUNDARY_PROOF_POLICY, CONTEXT_POLICY, LoopValidationSurface};

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
    if surface.hot_loop_policy == BOUNDARY_PROOF_POLICY {
        return MeasurementState {
            status: "withheld",
            failure_class: "boundary_proof_withheld_from_hot_loop",
            why_failed: format!(
                "{} is a strict boundary proof surface and is not executed as routine dirty hot-loop validation",
                surface.id
            ),
            where_failed: format!("loop.run.{}.boundary_mode", surface.id),
            next_repair: format!(
                "run `{}` at the strict claim boundary; do not substitute hot-loop validation for this proof surface",
                surface.canonical_full_command
            ),
            baseline_state: "strict_boundary_proof_not_required_for_hot_loop",
            speedup_state: "strict_boundary_proof_not_claimed_from_hot_loop",
            baseline_duration_ms: baseline_ms,
            speedup_ratio: baseline_ms.map(|value| value / duration_ms.max(1)),
            claim_impact: "boundary_claim_withheld_until_strict_proof_runs_on_current_candidate"
                .to_string(),
        };
    }
    if !surface.high_frequency {
        debug_assert_eq!(surface.hot_loop_policy, CONTEXT_POLICY);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::live_loop::surfaces::surface_by_id;

    #[test]
    fn boundary_surface_withholds_speed_claim_without_erasing_hot_loop_validation() {
        let state = measurement_state(
            surface_by_id("coverage_full_script").expect("coverage surface"),
            10,
            Some(100),
        );

        assert_eq!(state.status, "withheld");
        assert_eq!(state.failure_class, "boundary_proof_withheld_from_hot_loop");
        assert_eq!(state.speedup_ratio, Some(10));
        assert!(state.claim_impact.contains("boundary_claim_withheld"));
    }

    #[test]
    fn context_surface_is_observed_without_speed_or_closure_claim() {
        let state = measurement_state(
            surface_by_id("changed_files").expect("changed-files surface"),
            25,
            Some(100),
        );

        assert_eq!(state.status, "observed");
        assert_eq!(state.failure_class, "none");
        assert_eq!(state.speedup_ratio, Some(4));
        assert!(state.claim_impact.contains("observation_only_no_speed"));
    }

    #[test]
    fn high_frequency_surface_reports_pass_missed_and_missing_timing_separately() {
        let surface = surface_by_id("fmt_check").expect("fmt surface");
        let pass = measurement_state(surface, 2, Some(100));
        let missed = measurement_state(surface, 100, Some(1_000));
        let missing = measurement_state(surface, 2, None);

        assert_eq!(pass.status, "pass");
        assert_eq!(pass.speedup_ratio, Some(50));
        assert_eq!(missed.status, "blocked");
        assert_eq!(missed.failure_class, "live_loop_speedup_target_missed");
        assert_eq!(missed.speedup_ratio, Some(10));
        assert_eq!(missing.status, "blocked");
        assert_eq!(
            missing.failure_class,
            "live_loop_high_frequency_measurement_missing"
        );
        assert_eq!(missing.speedup_ratio, None);
    }
}
