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
        let validation_status = validation_status(verified_local);
        let validation_cache_status = validation_cache_status(verified_local);
        let observability_status = observability_status(verified_local);
        let speed_claim_status = speed_claim_status(
            surface,
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

fn validation_status(verified_local: &VerifiedLocalProof) -> &'static str {
    if verified_local.actual_work.launch_error {
        "blocked"
    } else if verified_local.actual_work.status_success {
        "pass"
    } else {
        "fail"
    }
}

fn validation_cache_status(verified_local: &VerifiedLocalProof) -> &'static str {
    if validation_status(verified_local) != "pass" {
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
    match failure_class {
        "none" => "supported",
        "live_loop_speedup_target_missed" => "failed",
        _ => "failed",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;
    use crate::cli::live_loop::nodes::measurement::full_command::FullCommandRun;
    use crate::cli::live_loop::nodes::measurement::observation::TelemetryReconciliation;
    use crate::cli::live_loop::surfaces::surface_by_id;

    #[test]
    fn validation_state_distinguishes_blocked_launch_from_failed_product_command() {
        let blocked = NodeTimingState::from_measurement(
            surface_by_id("fmt_check").expect("fmt surface"),
            &proof(|run| {
                run.launch_error = true;
                run.status_success = false;
                run.exit_code = 1;
            }),
            "canonical_full_command_failed",
        );
        assert_eq!(blocked.validation_status, "blocked");
        assert_eq!(blocked.validation_cache_status, "not_reusable");
        assert_eq!(blocked.speed_claim_status, "withheld");
        assert_eq!(blocked.claim_status, "withheld_validation_result_available");

        let failed = NodeTimingState::from_measurement(
            surface_by_id("fmt_check").expect("fmt surface"),
            &proof(|run| {
                run.status_success = false;
                run.exit_code = 2;
            }),
            "canonical_full_command_failed",
        );
        assert_eq!(failed.validation_status, "fail");
        assert_eq!(failed.validation_cache_status, "not_reusable");
        assert_eq!(failed.timing_status, "fail");
    }

    #[test]
    fn context_observation_does_not_claim_or_fail_speed() {
        let state = NodeTimingState::from_measurement(
            surface_by_id("changed_files").expect("changed files surface"),
            &proof(|_| {}),
            "live_loop_speedup_target_missed",
        );

        assert_eq!(state.validation_status, "pass");
        assert_eq!(state.observability_status, "pass");
        assert_eq!(state.speed_claim_status, "withheld");
        assert_eq!(state.timing_status, "partial");
        assert_eq!(state.claim_status, "withheld_validation_result_available");
    }

    fn proof(update: impl FnOnce(&mut FullCommandRun)) -> VerifiedLocalProof {
        let mut actual_work = FullCommandRun {
            exit_code: 0,
            status_success: true,
            launch_error: false,
            duration_ms: 1,
            stdout_digest: digest("stdout"),
            stderr_digest: digest("stderr"),
            failure: CommandFailureSummary::default(),
        };
        update(&mut actual_work);
        VerifiedLocalProof {
            proof_kind: "executed",
            cache_hit: false,
            cache_key: digest("cache"),
            graph_overhead_ms: 1,
            actual_work,
            work_unit_count: 1,
            equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
            invalidation_proof: "executed current command".to_string(),
            telemetry_reconciliation_status: "pass".to_string(),
            telemetry_reconciliation_duration_ms: 1,
            telemetry_reconciliation: TelemetryReconciliation {
                status: "pass".to_string(),
                duration_ms: 1,
                value: serde_json::json!({"status":"pass"}),
            },
            prior_result_digest: None,
            replayed_output_digest: None,
            cache_equivalence_status: None,
        }
    }

    fn digest(label: &str) -> String {
        crate::digest::bytes(label.as_bytes())
    }
}
