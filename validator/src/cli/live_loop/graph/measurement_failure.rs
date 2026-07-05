use super::super::nodes::status::MeasurementState;
use super::super::nodes::timing::NodeTiming;
use super::super::surfaces::LoopValidationSurface;

pub(crate) fn failed_timing_measurement_state(
    surface: LoopValidationSurface,
    timing: &NodeTiming,
) -> MeasurementState {
    let failure_class = match timing.failure_class.as_str() {
        "canonical_full_command_launch_failed" => "canonical_full_command_launch_failed",
        "canonical_full_command_failed" => "canonical_full_command_failed",
        "live_loop_speedup_target_missed" => "live_loop_speedup_target_missed",
        _ => "live_loop_node_measurement_failed",
    };
    let baseline_state = if timing.baseline_launch_error {
        "current_full_command_launch_failed"
    } else {
        "current_full_command_baseline_failed"
    };
    let fallback = fallback_why(failure_class, timing.baseline_launch_error);
    let where_failed = timing
        .baseline_failure
        .where_failed
        .clone()
        .unwrap_or_else(|| format!("loop.run.{}.baseline", surface.id));
    let next_repair = timing
        .baseline_failure
        .next_repair
        .clone()
        .unwrap_or_else(|| generic_repair(surface, timing));
    MeasurementState {
        status: "blocked",
        failure_class,
        why_failed: timing
            .baseline_failure
            .why_failed
            .clone()
            .unwrap_or_else(|| fallback.to_string()),
        where_failed,
        next_repair: format!(
            "{}; then rerun `target/debug/ultragoal --root . loop measure --node {} --tier hot --cache-mode verified-local`",
            next_repair, surface.id
        ),
        baseline_state,
        speedup_state: "verified_local_20x_not_claimable_until_full_command_passes",
        baseline_duration_ms: Some(timing.baseline_duration_ms),
        speedup_ratio: Some(timing.baseline_duration_ms / timing.verified_local_duration_ms.max(1)),
        claim_impact: timing
            .baseline_failure
            .claim_impact
            .clone()
            .unwrap_or_else(|| {
                "blocks_live_loop_routine_repair_until_canonical_full_command_passes".to_string()
            }),
    }
}

fn fallback_why(failure_class: &str, launch_error: bool) -> &'static str {
    if launch_error {
        "canonical full command could not launch while measuring this high-frequency live-loop node"
    } else if failure_class == "canonical_full_command_failed" {
        "canonical full command exited nonzero while measuring this high-frequency live-loop node"
    } else if failure_class == "live_loop_speedup_target_missed" {
        "high-frequency live-loop node has baseline timing but does not meet the verified-local 20x speed target"
    } else {
        "current high-frequency live-loop node timing row is failed"
    }
}

fn generic_repair(surface: LoopValidationSurface, timing: &NodeTiming) -> String {
    let exit = timing
        .baseline_exit_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "run `{}` and repair the failing command exit code {exit}",
        surface.canonical_full_command
    )
}
