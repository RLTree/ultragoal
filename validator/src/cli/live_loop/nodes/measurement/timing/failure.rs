use super::super::full_command::FullCommandRun;
use super::explain::telemetry_explain_text;
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::surfaces::LoopValidationSurface;

pub(crate) fn measurement_failure_class(
    baseline: &FullCommandRun,
    verified_local: &VerifiedLocalProof,
    speedup_ratio: u64,
) -> &'static str {
    if baseline.launch_error {
        "canonical_full_command_launch_failed"
    } else if !baseline.status_success {
        "canonical_full_command_failed"
    } else if verified_local.actual_work.launch_error {
        "verified_local_command_launch_failed"
    } else if !verified_local.actual_work.status_success {
        "verified_local_command_failed"
    } else if let Some(failure) = proof_kind_failure(verified_local) {
        failure
    } else if verified_local.telemetry_reconciliation_status != "pass" {
        "live_loop_telemetry_reconciliation_missing"
    } else if speedup_ratio < 20 {
        "live_loop_speedup_target_missed"
    } else {
        "none"
    }
}

fn proof_kind_failure(verified_local: &VerifiedLocalProof) -> Option<&'static str> {
    match verified_local.proof_kind {
        "executed" => executed_failure(verified_local),
        "verified_cache_hit" => cache_hit_failure(verified_local),
        _ => Some("verified_local_proof_kind_invalid"),
    }
}

fn executed_failure(verified_local: &VerifiedLocalProof) -> Option<&'static str> {
    if verified_local.cache_hit {
        Some("verified_local_cache_equivalence_missing")
    } else if verified_local.work_unit_count == 0 {
        Some("verified_local_work_unit_missing")
    } else if verified_local.equivalence_status != "executed_current_candidate_not_cache_replay" {
        Some("verified_local_equivalence_status_invalid")
    } else if verified_local.invalidation_proof.is_empty() {
        Some("verified_local_invalidation_proof_missing")
    } else {
        None
    }
}

fn cache_hit_failure(verified_local: &VerifiedLocalProof) -> Option<&'static str> {
    if !verified_local.cache_hit
        || verified_local.work_unit_count != 0
        || !valid_digest(verified_local.prior_result_digest.as_deref())
        || !valid_digest(verified_local.replayed_output_digest.as_deref())
        || verified_local.cache_equivalence_status.as_deref() != Some("pass")
    {
        Some("verified_local_cache_equivalence_missing")
    } else if verified_local.equivalence_status != "verified_same_candidate_cache_replay" {
        Some("verified_local_equivalence_status_invalid")
    } else if verified_local.invalidation_proof.is_empty() {
        Some("verified_local_invalidation_proof_missing")
    } else {
        None
    }
}

fn valid_digest(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 71
            && digest.starts_with("sha256:")
            && digest[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
pub(crate) fn measurement_where_failed(
    surface: LoopValidationSurface,
    baseline: &FullCommandRun,
    failure_class: &str,
) -> String {
    measurement_where_failed_base(surface, baseline, failure_class)
}

pub(crate) fn measurement_where_failed_with_telemetry(
    surface: LoopValidationSurface,
    baseline: &FullCommandRun,
    verified_local: &VerifiedLocalProof,
    failure_class: &str,
) -> String {
    if failure_class == "live_loop_telemetry_reconciliation_missing" {
        if let Some(where_failed) = telemetry_explain_text(verified_local, "where_failed") {
            return where_failed.to_string();
        }
    }
    measurement_where_failed_base(surface, baseline, failure_class)
}

fn measurement_where_failed_base(
    surface: LoopValidationSurface,
    baseline: &FullCommandRun,
    failure_class: &str,
) -> String {
    if matches!(
        failure_class,
        "canonical_full_command_launch_failed" | "canonical_full_command_failed"
    ) {
        return baseline
            .failure
            .where_failed
            .clone()
            .unwrap_or_else(|| format!("loop.measure.{}.canonical_full_command", surface.id));
    }
    let suffix = match failure_class {
        "verified_local_command_launch_failed" | "verified_local_command_failed" => {
            "verified_local_command"
        }
        "verified_local_work_unit_missing" => "work_unit",
        "verified_local_proof_kind_invalid" => "proof_kind",
        "verified_local_cache_equivalence_missing" => "cache_equivalence",
        "verified_local_equivalence_status_invalid" => "equivalence_status",
        "verified_local_invalidation_proof_missing" => "invalidation_proof",
        "live_loop_telemetry_reconciliation_missing" => "telemetry_reconciliation",
        "live_loop_speedup_target_missed" => "speedup",
        _ => "measurement",
    };
    format!("loop.measure.{}.{suffix}", surface.id)
}

#[cfg(test)]
pub(crate) fn measurement_why_failed(baseline: &FullCommandRun, failure_class: &str) -> String {
    measurement_why_failed_base(baseline, failure_class)
}

pub(crate) fn measurement_why_failed_with_telemetry(
    baseline: &FullCommandRun,
    verified_local: &VerifiedLocalProof,
    failure_class: &str,
) -> String {
    if failure_class == "live_loop_telemetry_reconciliation_missing" {
        if let Some(why_failed) = telemetry_explain_text(verified_local, "why_failed") {
            return why_failed.to_string();
        }
    }
    measurement_why_failed_base(baseline, failure_class)
}

fn measurement_why_failed_base(baseline: &FullCommandRun, failure_class: &str) -> String {
    if let Some(why) = baseline.failure.why_failed.clone().filter(|_| {
        matches!(
            failure_class,
            "canonical_full_command_launch_failed" | "canonical_full_command_failed"
        )
    }) {
        return why;
    }
    match failure_class {
        "none" => "none".to_string(),
        "canonical_full_command_launch_failed" => {
            "canonical full command could not launch while measuring live-loop node".to_string()
        }
        "canonical_full_command_failed" => {
            "canonical full command exited nonzero while measuring live-loop node".to_string()
        }
        "verified_local_command_launch_failed" => {
            "verified-local narrow command could not launch, so executed-work proof is missing"
                .to_string()
        }
        "verified_local_command_failed" => {
            "verified-local narrow command exited nonzero, so speed proof is not claimable"
                .to_string()
        }
        "verified_local_work_unit_missing" => {
            "speed row has no executed work unit and no verified cache equivalence".to_string()
        }
        "verified_local_proof_kind_invalid" => {
            "speed row proof_kind is not executed or verified same-candidate cache equivalence"
                .to_string()
        }
        "verified_local_cache_equivalence_missing" => {
            "speed row reports a cache hit without verified cache equivalence".to_string()
        }
        "verified_local_equivalence_status_invalid" => {
            "executed speed row lacks current-candidate execution equivalence status".to_string()
        }
        "verified_local_invalidation_proof_missing" => {
            "speed row lacks cache invalidation proof for the measured input".to_string()
        }
        "live_loop_telemetry_reconciliation_missing" => {
            "executed work lacks same-candidate log, metric, trace, and explain reconciliation"
                .to_string()
        }
        "live_loop_speedup_target_missed" => {
            "verified-local validation latency including graph overhead did not meet the 20x speed target".to_string()
        }
        _ => "live-loop node timing row failed strict proof validation".to_string(),
    }
}
