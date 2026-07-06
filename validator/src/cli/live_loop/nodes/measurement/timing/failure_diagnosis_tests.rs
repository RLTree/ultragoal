use super::failure::{
    measurement_failure_class, measurement_next_repair, measurement_where_failed,
    measurement_why_failed,
};
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;
use crate::cli::live_loop::nodes::measurement::full_command::FullCommandRun;
use crate::cli::live_loop::surfaces::surface_by_id;

#[test]
fn timing_failure_class_names_every_executed_work_blocker() {
    let cases = [
        (
            command_run(0, true, false, 100),
            proof_with(command_run(0, true, false, 1), |proof| {
                proof.proof_kind = "planned"
            }),
            100,
            "verified_local_proof_kind_invalid",
        ),
        (
            command_run(0, true, false, 100),
            proof_with(command_run(0, true, false, 1), |proof| {
                proof.cache_hit = true;
            }),
            100,
            "verified_local_cache_equivalence_missing",
        ),
        (
            command_run(0, true, false, 100),
            proof(command_run(0, true, true, 1), 1, "pass"),
            100,
            "verified_local_command_launch_failed",
        ),
        (
            command_run(0, true, false, 100),
            proof(command_run(2, false, false, 1), 1, "pass"),
            100,
            "verified_local_command_failed",
        ),
        (
            command_run(0, true, false, 100),
            proof(command_run(0, true, false, 1), 0, "pass"),
            100,
            "verified_local_work_unit_missing",
        ),
        (
            command_run(0, true, false, 100),
            proof_with(command_run(0, true, false, 1), |proof| {
                proof.equivalence_status = "unknown".to_string();
            }),
            100,
            "verified_local_equivalence_status_invalid",
        ),
        (
            command_run(0, true, false, 100),
            proof_with(command_run(0, true, false, 1), |proof| {
                proof.invalidation_proof = String::new();
            }),
            100,
            "verified_local_invalidation_proof_missing",
        ),
        (
            command_run(0, true, false, 100),
            proof(
                command_run(0, true, false, 1),
                1,
                "missing_query_reconciliation",
            ),
            100,
            "live_loop_telemetry_reconciliation_missing",
        ),
        (
            command_run(0, true, false, 100),
            proof(command_run(0, true, false, 10), 1, "pass"),
            10,
            "live_loop_speedup_target_missed",
        ),
    ];

    assert_eq!(
        measurement_failure_class(&command_run(1, false, true, 100), &cases[0].1, 100),
        "canonical_full_command_launch_failed"
    );
    assert_eq!(
        measurement_failure_class(&command_run(1, false, false, 100), &cases[0].1, 100),
        "canonical_full_command_failed"
    );
    assert!(
        measurement_why_failed(
            &command_run(1, false, true, 100),
            "canonical_full_command_launch_failed"
        )
        .contains("could not launch")
    );
    assert!(
        measurement_why_failed(
            &command_run(1, false, false, 100),
            "canonical_full_command_failed"
        )
        .contains("exited nonzero")
    );
    assert_eq!(
        measurement_where_failed(
            surface(),
            &command_run(1, false, false, 100),
            "canonical_full_command_failed"
        ),
        "loop.measure.changed_files.canonical_full_command"
    );
    assert!(
        measurement_next_repair(
            surface(),
            &command_run(1, false, false, 100),
            "canonical_full_command_failed"
        )
        .contains("git status --short --untracked-files=all")
    );
    for (baseline, verified, ratio, expected) in cases {
        assert_eq!(
            measurement_failure_class(&baseline, &verified, ratio),
            expected
        );
        assert_ne!(
            measurement_where_failed(surface(), &baseline, expected),
            "none"
        );
        assert_ne!(measurement_why_failed(&baseline, expected), "none");
        assert_ne!(
            measurement_next_repair(surface(), &baseline, expected),
            "none"
        );
    }
    assert!(
        measurement_why_failed(&command_run(0, true, false, 100), "unexpected_failure")
            .contains("strict proof validation")
    );
    assert!(
        measurement_next_repair(
            surface(),
            &command_run(0, true, false, 100),
            "unexpected_failure"
        )
        .contains("timing receipt fields")
    );
}

#[test]
fn timing_failure_output_prefers_command_failure_details_when_baseline_failed() {
    let mut baseline = command_run(101, false, false, 100);
    baseline.failure.why_failed = Some("coverage digest stale".to_string());
    baseline.failure.where_failed = Some("coverage.prove.digest".to_string());
    baseline.failure.next_repair = Some("refresh coverage".to_string());

    assert_eq!(
        measurement_where_failed(surface(), &baseline, "canonical_full_command_failed"),
        "coverage.prove.digest"
    );
    assert_eq!(
        measurement_why_failed(&baseline, "canonical_full_command_failed"),
        "coverage digest stale"
    );
    assert_eq!(
        measurement_next_repair(surface(), &baseline, "canonical_full_command_failed"),
        "refresh coverage"
    );
}

fn surface() -> crate::cli::live_loop::surfaces::LoopValidationSurface {
    surface_by_id("changed_files").expect("surface")
}

fn command_run(
    exit_code: i32,
    status_success: bool,
    launch_error: bool,
    duration_ms: u64,
) -> FullCommandRun {
    FullCommandRun {
        exit_code,
        status_success,
        launch_error,
        duration_ms,
        stdout_digest: "sha256:stdout".to_string(),
        stderr_digest: "sha256:stderr".to_string(),
        failure: CommandFailureSummary::default(),
    }
}

fn proof(
    actual_work: FullCommandRun,
    work_unit_count: u64,
    telemetry_reconciliation_status: &'static str,
) -> VerifiedLocalProof {
    VerifiedLocalProof {
        proof_kind: "executed",
        cache_hit: false,
        cache_key: "sha256:cache".to_string(),
        graph_overhead_ms: 1,
        actual_work,
        work_unit_count,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: telemetry_reconciliation_status.to_string(),
        telemetry_reconciliation_duration_ms: 1,
        telemetry_reconciliation: super::super::observation::TelemetryReconciliation {
            status: telemetry_reconciliation_status.to_string(),
            duration_ms: 1,
            value: serde_json::json!({"status": telemetry_reconciliation_status}),
        },
        prior_result_digest: None,
        replayed_output_digest: None,
        cache_equivalence_status: None,
    }
}

fn proof_with(
    actual_work: FullCommandRun,
    update: impl FnOnce(&mut VerifiedLocalProof),
) -> VerifiedLocalProof {
    let mut proof = proof(actual_work, 1, "pass");
    update(&mut proof);
    proof
}
