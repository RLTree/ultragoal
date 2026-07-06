use crate::cli::live_loop::{LiveLoopAction, LiveLoopCommand};
use std::path::{Path, PathBuf};

pub(super) const LIVE_LOOP_TIMING_RECEIPT: &str =
    "validation_artifacts/observability/live-loop-node-timing.json";

pub(super) fn command(node_id: Option<&str>, receipt: PathBuf) -> LiveLoopCommand {
    LiveLoopCommand {
        action: LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt,
        node_id: node_id.map(str::to_string),
        measure_all: false,
    }
}

pub(super) fn live_loop_timing_receipt_arg() -> PathBuf {
    Path::new(LIVE_LOOP_TIMING_RECEIPT).to_path_buf()
}

pub(super) fn live_loop_timing_receipt_path(root: &std::path::Path) -> PathBuf {
    crate::output_path::literal_claim_artifact_path(
        root,
        LIVE_LOOP_TIMING_RECEIPT,
        "node timing receipt",
    )
}

pub(super) fn full_command_run(
    exit_code: i32,
    status_success: bool,
    duration_ms: u64,
) -> super::super::full_command::FullCommandRun {
    super::super::full_command::FullCommandRun {
        exit_code,
        status_success,
        launch_error: false,
        duration_ms,
        stdout_digest: "sha256:stdout".to_string(),
        stderr_digest: "sha256:stderr".to_string(),
        failure: Default::default(),
    }
}

pub(super) fn verified_local_proof(
    exit_code: i32,
    status_success: bool,
    duration_ms: u64,
    telemetry_reconciliation_status: &'static str,
) -> super::super::timing::verified_work::VerifiedLocalProof {
    super::super::timing::verified_work::VerifiedLocalProof {
        proof_kind: "executed",
        cache_hit: false,
        cache_key: "sha256:cache".to_string(),
        graph_overhead_ms: 1,
        actual_work: full_command_run(exit_code, status_success, duration_ms),
        work_unit_count: 1,
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
