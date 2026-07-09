use super::record::{MeasurementExecutionAuthority, node_timing_row};
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;
use crate::cli::live_loop::nodes::measurement::full_command::FullCommandRun;
use crate::cli::live_loop::nodes::measurement::observation::TelemetryReconciliation;
use crate::cli::live_loop::surfaces::surface_by_id;
use crate::cli::live_loop::{LiveLoopAction, LiveLoopCommand};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn measurement_rust_tests_zero_test_execution_is_not_cacheable_validation() {
    let surface =
        surface_by_id("live_loop_measurement_rust_tests").expect("measurement test surface");
    let command = LiveLoopCommand {
        action: LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: PathBuf::from(super::super::super::timing::NODE_TIMING_REL),
        node_id: Some(surface.id.to_string()),
        measure_all: false,
    };
    let run = FullCommandRun {
        exit_code: 0,
        status_success: true,
        launch_error: false,
        duration_ms: 1,
        stdout_digest: digest("stdout"),
        stderr_digest: digest("stderr"),
        executed_test_count: None,
        failure: CommandFailureSummary::default(),
    };
    let proof = VerifiedLocalProof {
        proof_kind: "executed",
        cache_hit: false,
        cache_key: digest("cache"),
        graph_overhead_ms: 1,
        actual_work: run.clone(),
        work_unit_count: 1,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        telemetry_reconciliation_duration_ms: 1,
        telemetry_reconciliation: TelemetryReconciliation {
            status: "pass".to_string(),
            duration_ms: 1,
            value: json!({"status":"pass"}),
        },
        prior_result_digest: None,
        replayed_output_digest: None,
        cache_equivalence_status: None,
        source_speed_claim_status: None,
        routine_replay_speed_claim_status: None,
    };
    let row = node_timing_row(
        surface,
        &command,
        &digest("candidate"),
        &digest("changed"),
        &digest("audit"),
        &digest("input"),
        &run,
        &proof,
        "changed_inputs_present",
        &MeasurementExecutionAuthority::single_surface(surface),
    );

    assert!(row.blocks_hot_loop());
    assert_eq!(row["failure_class"], "verified_local_zero_tests_executed");
    assert_eq!(row["validation_status"], "fail");
    assert_eq!(row["validation_cache_status"], "not_reusable");
    assert_eq!(row["speed_claim_status"], "withheld");
    assert_eq!(row["verified_local_executed_test_count"], 0);
}

fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}
