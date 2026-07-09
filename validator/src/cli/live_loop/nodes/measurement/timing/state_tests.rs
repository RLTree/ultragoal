use super::state::NodeTimingState;
use super::verified_work::VerifiedLocalProof;
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

#[test]
fn withheld_source_cache_replay_needs_explicit_speed_eligibility() {
    let mut replay = proof(|_| {});
    replay.proof_kind = "verified_cache_hit";
    replay.cache_hit = true;
    replay.work_unit_count = 0;
    replay.equivalence_status = "verified_same_candidate_cache_replay".to_string();
    replay.invalidation_proof =
        "cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched"
            .to_string();
    replay.prior_result_digest = Some(digest("prior"));
    replay.replayed_output_digest = Some(digest("replayed"));
    replay.cache_equivalence_status = Some("pass".to_string());
    replay.source_speed_claim_status = Some("withheld".to_string());

    let without_eligibility = NodeTimingState::from_measurement(
        surface_by_id("line_caps_check").expect("line cap surface"),
        &replay,
        "none",
    );
    assert_eq!(without_eligibility.validation_cache_status, "reusable");
    assert_eq!(without_eligibility.speed_claim_status, "withheld");

    replay.routine_replay_speed_claim_status =
        Some("eligible_after_verified_cache_hit".to_string());
    let eligible = NodeTimingState::from_measurement(
        surface_by_id("line_caps_check").expect("line cap surface"),
        &replay,
        "none",
    );
    assert_eq!(eligible.speed_claim_status, "supported");
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
        source_speed_claim_status: None,
        routine_replay_speed_claim_status: None,
    }
}

fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}
