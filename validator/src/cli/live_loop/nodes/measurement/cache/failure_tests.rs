use super::super::failure::measurement_failure_class;
use super::super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;
use crate::cli::live_loop::nodes::measurement::full_command::FullCommandRun;

#[test]
fn verified_cache_hit_failure_classes_require_equivalence_and_invalidation() {
    let baseline = command_run(100);
    let cases = [
        (
            cache_hit_proof(|proof| {
                proof.cache_equivalence_status = None;
            }),
            "verified_local_cache_equivalence_missing",
        ),
        (
            cache_hit_proof(|proof| {
                proof.prior_result_digest = Some("sha256:prior".to_string());
            }),
            "verified_local_cache_equivalence_missing",
        ),
        (
            cache_hit_proof(|proof| {
                proof.equivalence_status = "unknown".to_string();
            }),
            "verified_local_equivalence_status_invalid",
        ),
        (
            cache_hit_proof(|proof| {
                proof.invalidation_proof = String::new();
            }),
            "verified_local_invalidation_proof_missing",
        ),
        (cache_hit_proof(|_| {}), "none"),
    ];

    for (proof, expected) in cases {
        assert_eq!(measurement_failure_class(&baseline, &proof, 100), expected);
    }
}

fn cache_hit_proof(update: impl FnOnce(&mut VerifiedLocalProof)) -> VerifiedLocalProof {
    let mut proof = VerifiedLocalProof {
        proof_kind: "verified_cache_hit",
        cache_hit: true,
        cache_key: digest("cache"),
        graph_overhead_ms: 1,
        actual_work: command_run(1),
        work_unit_count: 0,
        equivalence_status: "verified_same_candidate_cache_replay".to_string(),
        invalidation_proof:
            "cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched"
                .to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        telemetry_reconciliation_duration_ms: 1,
        telemetry_reconciliation: super::super::super::observation::TelemetryReconciliation {
            status: "pass".to_string(),
            duration_ms: 1,
            value: serde_json::json!({"status": "pass"}),
        },
        prior_result_digest: Some(digest("prior")),
        replayed_output_digest: Some(digest("replayed")),
        cache_equivalence_status: Some("pass".to_string()),
        source_speed_claim_status: Some("supported".to_string()),
        routine_replay_speed_claim_status: None,
    };
    update(&mut proof);
    proof
}

fn command_run(duration_ms: u64) -> FullCommandRun {
    FullCommandRun {
        exit_code: 0,
        status_success: true,
        launch_error: false,
        duration_ms,
        stdout_digest: digest("stdout"),
        stderr_digest: digest("stderr"),
        executed_test_count: None,
        failure: CommandFailureSummary::default(),
    }
}

fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}
