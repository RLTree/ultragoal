use super::super::full_command::FullCommandRun;
use super::explain::telemetry_explain_text;
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::nodes::timing::NODE_TIMING_REL;
use crate::cli::live_loop::surfaces::LoopValidationSurface;

#[cfg(test)]
pub(crate) fn measurement_next_repair(
    surface: LoopValidationSurface,
    baseline: &FullCommandRun,
    failure_class: &str,
) -> String {
    measurement_next_repair_base(surface, baseline, failure_class)
}

pub(crate) fn measurement_next_repair_with_telemetry(
    surface: LoopValidationSurface,
    baseline: &FullCommandRun,
    verified_local: &VerifiedLocalProof,
    failure_class: &str,
) -> String {
    if failure_class == "live_loop_telemetry_reconciliation_missing" {
        if let Some(next_repair) = telemetry_explain_text(verified_local, "next_repair") {
            return next_repair.to_string();
        }
    }
    measurement_next_repair_base(surface, baseline, failure_class)
}

fn measurement_next_repair_base(
    surface: LoopValidationSurface,
    baseline: &FullCommandRun,
    failure_class: &str,
) -> String {
    if let Some(next) = baseline.failure.next_repair.clone().filter(|_| {
        matches!(
            failure_class,
            "canonical_full_command_launch_failed" | "canonical_full_command_failed"
        )
    }) {
        return next;
    }
    match failure_class {
        "none" => "none".to_string(),
        "canonical_full_command_launch_failed" | "canonical_full_command_failed" => format!(
            "run `{}` directly, repair the command failure, then rerun `target/debug/ultragoal --root . loop measure --node {} --tier hot --cache-mode verified-local`",
            surface.canonical_full_command, surface.id
        ),
        "verified_local_command_launch_failed" | "verified_local_command_failed" => format!(
            "run `{}` directly, repair the narrow command behavior, then rerun `target/debug/ultragoal --root . loop measure --node {} --tier hot --cache-mode verified-local`",
            surface.narrow_rerun, surface.id
        ),
        "verified_local_zero_tests_executed" => format!(
            "repair `{}` so it executes at least one Rust test before caching or claiming validation, then rerun node `{}`",
            surface.narrow_rerun, surface.id
        ),
        "verified_local_work_unit_missing" => format!(
            "execute `{}` or provide verified same-candidate cache equivalence before recomputing the speed row",
            surface.narrow_rerun
        ),
        "verified_local_proof_kind_invalid" => format!(
            "replace proof-shaped timing for `{}` with executed work or verified same-candidate cache equivalence, then rerun node `{}`",
            surface.narrow_rerun, surface.id
        ),
        "verified_local_cache_equivalence_missing" => format!(
            "verify cache key, current input digests, prior result digest, replayed output digest, and invalidation proof before treating `{}` as a cache hit",
            surface.narrow_rerun
        ),
        "verified_local_equivalence_status_invalid" => format!(
            "record current-candidate execution equivalence for `{}` or fail the timing row",
            surface.narrow_rerun
        ),
        "verified_local_invalidation_proof_missing" => format!(
            "record input invalidation proof for `{}` before claiming loop speed",
            surface.narrow_rerun
        ),
        "live_loop_telemetry_reconciliation_missing" => format!(
            "bind `{}` to same-candidate logs, metrics, traces, and explain output, then rerun `target/debug/ultragoal --root . loop measure --node {} --tier hot --cache-mode verified-local`",
            surface.narrow_rerun, surface.id
        ),
        "live_loop_speedup_target_missed" => format!(
            "split, cache, or daemonize `{}` with verified equivalence until node `{}` validation latency is at least 20x faster than its canonical baseline",
            surface.narrow_rerun, surface.id
        ),
        _ => format!(
            "inspect `{}` timing receipt fields, repair missing proof data, then rerun node `{}`",
            NODE_TIMING_REL, surface.id
        ),
    }
}
