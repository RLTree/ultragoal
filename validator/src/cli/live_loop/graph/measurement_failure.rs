use super::super::nodes::status::MeasurementState;
use super::super::nodes::timing::NodeTiming;
use super::super::surfaces::LoopValidationSurface;

pub(crate) fn failed_timing_measurement_state(
    surface: LoopValidationSurface,
    timing: &NodeTiming,
) -> MeasurementState {
    let failure_class = measurement_failure_class(timing);
    let baseline_state = baseline_state(failure_class, timing);
    let speedup_state = speedup_state(failure_class);
    let fallback = fallback_why(failure_class, timing.baseline_launch_error);
    let where_failed = specific_or_default(
        &timing.where_failed,
        || timing.baseline_failure.where_failed.clone(),
        || where_failed_for(surface, failure_class),
    );
    let next_repair = specific_or_default(
        &timing.next_repair,
        || timing.baseline_failure.next_repair.clone(),
        || generic_repair(surface, timing),
    );
    let why_failed = specific_or_default(
        &timing.why_failed,
        || timing.baseline_failure.why_failed.clone(),
        || fallback.to_string(),
    );
    let status = if timing.validation_status == "pass" && timing.speed_claim_status == "withheld" {
        "partial"
    } else {
        "blocked"
    };
    MeasurementState {
        status,
        failure_class,
        why_failed,
        where_failed,
        next_repair: rerun_suffix(next_repair, surface),
        baseline_state,
        speedup_state,
        baseline_duration_ms: Some(timing.baseline_duration_ms),
        speedup_ratio: Some(
            timing.baseline_duration_ms / timing.reconciled_command_duration_ms.max(1),
        ),
        claim_impact: claim_impact(timing),
    }
}

fn measurement_failure_class(timing: &NodeTiming) -> &'static str {
    match timing.failure_class.as_str() {
        "none" if timing.telemetry_reconciliation_status != "pass" => {
            "live_loop_telemetry_reconciliation_missing"
        }
        "canonical_full_command_launch_failed" => "canonical_full_command_launch_failed",
        "canonical_full_command_failed" => "canonical_full_command_failed",
        "verified_local_command_launch_failed" => "verified_local_command_launch_failed",
        "verified_local_command_failed" => "verified_local_command_failed",
        "verified_local_proof_kind_invalid" => "verified_local_proof_kind_invalid",
        "verified_local_cache_equivalence_missing" => "verified_local_cache_equivalence_missing",
        "verified_local_work_unit_missing" => "verified_local_work_unit_missing",
        "verified_local_equivalence_status_invalid" => "verified_local_equivalence_status_invalid",
        "verified_local_invalidation_proof_missing" => "verified_local_invalidation_proof_missing",
        "live_loop_telemetry_reconciliation_missing" => {
            "live_loop_telemetry_reconciliation_missing"
        }
        "live_loop_speedup_target_missed" => "live_loop_speedup_target_missed",
        _ => "live_loop_node_measurement_failed",
    }
}

fn rerun_suffix(next_repair: String, surface: LoopValidationSurface) -> String {
    let rerun = format!(
        "target/debug/ultragoal --root . loop measure --node {}",
        surface.id
    );
    if next_repair.contains(&rerun) {
        next_repair
    } else {
        format!("{next_repair}; then rerun `{rerun} --tier hot --cache-mode verified-local`")
    }
}

fn specific_or_default(
    value: &str,
    fallback: impl FnOnce() -> Option<String>,
    default: impl FnOnce() -> String,
) -> String {
    if !value.is_empty() && value != "none" {
        value.to_string()
    } else {
        fallback().unwrap_or_else(default)
    }
}

fn fallback_why(failure_class: &str, launch_error: bool) -> &'static str {
    if launch_error {
        "canonical full command could not launch while measuring this high-frequency live-loop node"
    } else if failure_class == "canonical_full_command_failed" {
        "canonical full command exited nonzero while measuring this high-frequency live-loop node"
    } else if failure_class == "verified_local_command_launch_failed" {
        "verified-local live-loop command could not launch, so no executed work can support a speed claim"
    } else if failure_class == "verified_local_command_failed" {
        "verified-local live-loop command exited nonzero, so no speed claim can be made for this node"
    } else if failure_class == "verified_local_proof_kind_invalid" {
        "live-loop timing row uses proof-shaped output instead of executed work or verified cache equivalence"
    } else if failure_class == "verified_local_cache_equivalence_missing" {
        "live-loop timing row reports cache reuse without same-candidate equivalence proof"
    } else if failure_class == "verified_local_work_unit_missing" {
        "verified-local live-loop timing row has no executed work unit and no verified cache equivalence"
    } else if failure_class == "verified_local_equivalence_status_invalid" {
        "executed live-loop timing row lacks current-candidate execution equivalence status"
    } else if failure_class == "verified_local_invalidation_proof_missing" {
        "live-loop timing row lacks cache invalidation proof for the measured input"
    } else if failure_class == "live_loop_telemetry_reconciliation_missing" {
        "verified-local live-loop timing row lacks same-candidate telemetry reconciliation for the executed work"
    } else if failure_class == "live_loop_speedup_target_missed" {
        "high-frequency live-loop node has baseline timing but does not meet the verified-local 20x speed target"
    } else {
        "current high-frequency live-loop node timing row is failed"
    }
}

fn baseline_state(failure_class: &str, timing: &NodeTiming) -> &'static str {
    match failure_class {
        "canonical_full_command_launch_failed" if timing.baseline_launch_error => {
            "current_full_command_launch_failed"
        }
        "canonical_full_command_failed" => "current_full_command_baseline_failed",
        _ => "current_full_command_baseline_observed",
    }
}

fn speedup_state(failure_class: &str) -> &'static str {
    match failure_class {
        "canonical_full_command_launch_failed" | "canonical_full_command_failed" => {
            "verified_local_20x_not_claimable_until_full_command_passes"
        }
        "verified_local_command_launch_failed" => "verified_local_command_launch_failed",
        "verified_local_command_failed" => "verified_local_command_failed",
        "verified_local_proof_kind_invalid" => "verified_local_proof_kind_invalid",
        "verified_local_cache_equivalence_missing" => "verified_local_cache_equivalence_missing",
        "verified_local_work_unit_missing" => "verified_local_work_unit_missing",
        "verified_local_equivalence_status_invalid" => "verified_local_equivalence_status_invalid",
        "verified_local_invalidation_proof_missing" => "verified_local_invalidation_proof_missing",
        "live_loop_telemetry_reconciliation_missing" => "telemetry_reconciliation_missing",
        "live_loop_speedup_target_missed" => "verified_local_20x_proof_failed",
        _ => "verified_local_measurement_failed",
    }
}

fn claim_impact(timing: &NodeTiming) -> String {
    if timing.validation_status == "pass" && timing.speed_claim_status == "withheld" {
        return "source_local_validation_available_speed_or_observability_claim_withheld"
            .to_string();
    }
    timing
        .baseline_failure
        .claim_impact
        .clone()
        .unwrap_or_else(|| {
            "blocks_live_loop_routine_repair_until_canonical_full_command_passes".to_string()
        })
}

fn where_failed_for(surface: LoopValidationSurface, failure_class: &str) -> String {
    let suffix = match failure_class {
        "canonical_full_command_launch_failed" | "canonical_full_command_failed" => "baseline",
        "verified_local_command_launch_failed" | "verified_local_command_failed" => {
            "verified_local"
        }
        "verified_local_proof_kind_invalid" => "proof_kind",
        "verified_local_cache_equivalence_missing" => "cache_equivalence",
        "verified_local_work_unit_missing" => "work_unit",
        "verified_local_equivalence_status_invalid" => "equivalence_status",
        "verified_local_invalidation_proof_missing" => "invalidation_proof",
        "live_loop_telemetry_reconciliation_missing" => "telemetry_reconciliation",
        "live_loop_speedup_target_missed" => "speedup",
        _ => "measurement",
    };
    format!("loop.run.{}.{suffix}", surface.id)
}

fn generic_repair(surface: LoopValidationSurface, timing: &NodeTiming) -> String {
    if timing.baseline_launch_error {
        return format!(
            "make `{}` launch successfully, then rerun it directly to capture baseline timing",
            surface.canonical_full_command
        );
    }
    let exit = timing
        .baseline_exit_code
        .expect("baseline exit code is required when the canonical command launched");
    format!(
        "run `{}` and repair the failing command exit code {exit}",
        surface.canonical_full_command
    )
}
