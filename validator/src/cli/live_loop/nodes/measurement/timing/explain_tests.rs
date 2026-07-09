use super::failure::{
    measurement_where_failed_with_telemetry, measurement_why_failed_with_telemetry,
};
use super::repair::measurement_next_repair_with_telemetry;
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;
use crate::cli::live_loop::nodes::measurement::full_command::FullCommandRun;
use crate::cli::live_loop::nodes::measurement::observation;
use crate::cli::live_loop::surfaces::surface_by_id;

#[test]
fn timing_failure_output_uses_explain_details_when_telemetry_reconciliation_failed() {
    let baseline = command_run(0, true, false, 100);
    let verified = proof_with(command_run(0, true, false, 1), |proof| {
        proof.telemetry_reconciliation_status =
            "query_or_explain_reconciliation_failed".to_string();
        proof.telemetry_reconciliation.value = serde_json::json!({
            "status": "query_or_explain_reconciliation_failed",
            "explain_failure": {
                "value": {
                    "where_failed": "observe.metrics.query",
                    "why_failed": "observability_metric_event_time_stale:metric_event_unix=1 target_event_unix=2",
                    "next_repair": "rerun observe metrics query by run_id/correlation_id/current digest"
                }
            }
        });
    });

    assert_eq!(
        measurement_where_failed_with_telemetry(
            surface(),
            &baseline,
            &verified,
            "live_loop_telemetry_reconciliation_missing"
        ),
        "observe.metrics.query"
    );
    assert!(
        measurement_why_failed_with_telemetry(
            &baseline,
            &verified,
            "live_loop_telemetry_reconciliation_missing"
        )
        .contains("observability_metric_event_time_stale")
    );
    assert_eq!(
        measurement_next_repair_with_telemetry(
            surface(),
            &baseline,
            &verified,
            "live_loop_telemetry_reconciliation_missing"
        ),
        "rerun observe metrics query by run_id/correlation_id/current digest"
    );
}

#[test]
fn timing_failure_output_uses_cached_explain_details_when_replaying_telemetry() {
    let baseline = command_run(0, true, false, 100);
    let verified = proof_with(command_run(0, true, false, 1), |proof| {
        proof.proof_kind = "verified_cache_hit";
        proof.cache_hit = true;
        proof.work_unit_count = 0;
        proof.telemetry_reconciliation_status =
            "query_or_explain_reconciliation_failed".to_string();
        proof.telemetry_reconciliation.value = serde_json::json!({
            "status": "query_or_explain_reconciliation_failed",
            "reconciliation_mode": "verified_same_candidate_telemetry_reuse",
            "cached_reconciliation": {
                "status": "query_or_explain_reconciliation_failed",
                "explain_failure": {
                    "value": {
                        "where_failed": "observe.traces.query",
                        "why_failed": "observability_trace_parentage_missing:trace_id=trace-cached",
                        "next_repair": "rerun observe traces query by run_id/correlation_id/current digest"
                    }
                }
            }
        });
    });

    assert_eq!(
        measurement_where_failed_with_telemetry(
            surface(),
            &baseline,
            &verified,
            "live_loop_telemetry_reconciliation_missing"
        ),
        "observe.traces.query"
    );
    assert!(
        measurement_why_failed_with_telemetry(
            &baseline,
            &verified,
            "live_loop_telemetry_reconciliation_missing"
        )
        .contains("observability_trace_parentage_missing")
    );
    assert_eq!(
        measurement_next_repair_with_telemetry(
            surface(),
            &baseline,
            &verified,
            "live_loop_telemetry_reconciliation_missing"
        ),
        "rerun observe traces query by run_id/correlation_id/current digest"
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

fn proof_with(
    actual_work: FullCommandRun,
    update: impl FnOnce(&mut VerifiedLocalProof),
) -> VerifiedLocalProof {
    let mut proof = VerifiedLocalProof {
        proof_kind: "executed",
        cache_hit: false,
        cache_key: "sha256:cache".to_string(),
        graph_overhead_ms: 1,
        actual_work,
        work_unit_count: 1,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        telemetry_reconciliation_duration_ms: 1,
        telemetry_reconciliation: observation::TelemetryReconciliation {
            status: "pass".to_string(),
            duration_ms: 1,
            value: serde_json::json!({"status": "pass"}),
        },
        prior_result_digest: None,
        replayed_output_digest: None,
        cache_equivalence_status: None,
        source_speed_claim_status: None,
        routine_replay_speed_claim_status: None,
    };
    update(&mut proof);
    proof
}
