use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::surfaces::LoopValidationSurface;

pub(crate) struct NodeTimingState {
    pub(crate) validation_status: &'static str,
    pub(crate) validation_cache_status: &'static str,
    pub(crate) observability_status: &'static str,
    pub(crate) speed_claim_status: &'static str,
    pub(crate) timing_status: &'static str,
    pub(crate) observability_failure_class: &'static str,
    pub(crate) claim_status: &'static str,
}

impl NodeTimingState {
    pub(crate) fn from_measurement(
        surface: LoopValidationSurface,
        verified_local: &VerifiedLocalProof,
        failure_class: &str,
    ) -> Self {
        let validation_status = validation_status(verified_local, failure_class);
        let validation_cache_status = validation_cache_status(verified_local, failure_class);
        let observability_status = observability_status(verified_local);
        let speed_claim_status = speed_claim_status(
            surface,
            verified_local,
            validation_status,
            observability_status,
            failure_class,
        );
        Self {
            validation_status,
            validation_cache_status,
            observability_status,
            speed_claim_status,
            timing_status: timing_status(validation_status, speed_claim_status),
            observability_failure_class: observability_failure_class(verified_local),
            claim_status: claim_status(speed_claim_status),
        }
    }
}

pub(crate) fn timing_status(validation_status: &str, speed_claim_status: &str) -> &'static str {
    match (validation_status, speed_claim_status) {
        ("pass", "supported") => "pass",
        ("pass", "withheld") => "partial",
        _ => "fail",
    }
}

fn validation_status(verified_local: &VerifiedLocalProof, failure_class: &str) -> &'static str {
    if verified_local.actual_work.launch_error {
        "blocked"
    } else if failure_class == "verified_local_zero_tests_executed" {
        "fail"
    } else if verified_local.actual_work.status_success {
        "pass"
    } else {
        "fail"
    }
}

fn validation_cache_status(
    verified_local: &VerifiedLocalProof,
    failure_class: &str,
) -> &'static str {
    if validation_status(verified_local, failure_class) != "pass" {
        return "not_reusable";
    }
    match verified_local.proof_kind {
        "executed"
            if !verified_local.cache_hit
                && verified_local.work_unit_count > 0
                && verified_local.equivalence_status
                    == "executed_current_candidate_not_cache_replay"
                && !verified_local.invalidation_proof.is_empty() =>
        {
            "reusable"
        }
        "verified_cache_hit"
            if verified_local.cache_hit
                && verified_local.work_unit_count == 0
                && verified_local.equivalence_status == "verified_same_candidate_cache_replay"
                && !verified_local.invalidation_proof.is_empty()
                && verified_local.cache_equivalence_status.as_deref() == Some("pass")
                && verified_local
                    .prior_result_digest
                    .as_deref()
                    .is_some_and(valid_digest)
                && verified_local
                    .replayed_output_digest
                    .as_deref()
                    .is_some_and(valid_digest) =>
        {
            "reusable"
        }
        _ => "unknown",
    }
}

fn observability_status(verified_local: &VerifiedLocalProof) -> &'static str {
    match verified_local.telemetry_reconciliation_status.as_str() {
        "pass" => "pass",
        "command_observation_failed" => "unavailable",
        _ => "partial",
    }
}

fn speed_claim_status(
    surface: LoopValidationSurface,
    verified_local: &VerifiedLocalProof,
    validation_status: &str,
    observability_status: &str,
    failure_class: &str,
) -> &'static str {
    if validation_status != "pass" || observability_status != "pass" {
        return "withheld";
    }
    if !surface.high_frequency {
        return "withheld";
    }
    if !routine_cache_replay_carries_speed_claim_authority(verified_local) {
        return "withheld";
    }
    match failure_class {
        "none" => "supported",
        "live_loop_speedup_target_missed" => "failed",
        _ => "failed",
    }
}

fn routine_cache_replay_carries_speed_claim_authority(verified_local: &VerifiedLocalProof) -> bool {
    if verified_local.proof_kind != "verified_cache_hit" {
        return true;
    }
    match verified_local.source_speed_claim_status.as_deref() {
        Some("supported" | "failed") => true,
        Some("withheld") => {
            verified_local.routine_replay_speed_claim_status.as_deref()
                == Some("eligible_after_verified_cache_hit")
        }
        _ => false,
    }
}

fn observability_failure_class(verified_local: &VerifiedLocalProof) -> &'static str {
    match verified_local.telemetry_reconciliation_status.as_str() {
        "pass" => "none",
        "command_observation_failed" => "live_loop_observability_unavailable",
        _ => "live_loop_observability_partial",
    }
}

fn claim_status(speed_claim_status: &str) -> &'static str {
    match speed_claim_status {
        "supported" => "supported_source_local",
        "withheld" => "withheld_validation_result_available",
        _ => "blocked",
    }
}

fn valid_digest(digest: &str) -> bool {
    digest.len() == 71
        && digest.starts_with("sha256:")
        && digest[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}
