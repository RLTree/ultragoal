use super::*;
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use crate::scheduler::TaskClass;
use std::path::PathBuf;

fn command_for(tier: &str, cache_mode: &str) -> LiveLoopCommand {
    LiveLoopCommand {
        action: crate::cli::live_loop::LiveLoopAction::Measure,
        tier: tier.to_string(),
        cache_mode: cache_mode.to_string(),
        jobs: None,
        receipt: PathBuf::from("validation_artifacts/observability/live-loop-node-timing.json"),
        node_id: Some("unit_measure_branch".to_string()),
        measure_all: false,
    }
}

fn product_surface(
    id: &'static str,
    canonical_full_command: &'static str,
    narrow_rerun: &'static str,
) -> LoopValidationSurface {
    LoopValidationSurface {
        id,
        surface: "rust_validation",
        command: "live loop snapshot branch contract",
        canonical_full_command,
        narrow_rerun,
        telemetry_reconciliation_state: "requires_command_telemetry_roundtrip",
        execution_task_class: TaskClass::PureReadParallel,
        execution_serial_reason: "none",
        high_frequency: true,
        hot_loop_policy: "routine_hot_repair",
    }
}

fn inputs() -> ChangedInputs {
    ChangedInputs::for_tests(
        &crate::digest::bytes(b"changed"),
        &crate::digest::bytes(b"audit"),
    )
}

#[test]
fn loop_run_snapshot_keeps_validation_result_when_observability_roundtrip_is_pending() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-run-snapshot-validation-state",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = product_surface(
        "unit_loop_run_snapshot",
        "printf same-command",
        "printf same-command",
    );

    let row = measure_surface(
        &root,
        &command_for("hot", "verified-local"),
        surface,
        "sha256:current",
        &inputs(),
        "changed_files_digest_bound",
        ObservationMode::LoopRunSnapshot,
    );

    assert_eq!(row["validation_status"], "pass");
    assert_eq!(row["validation_cache_status"], "reusable");
    assert_eq!(row["observability_status"], "partial");
    assert_eq!(row["speed_claim_status"], "withheld");
    assert_eq!(
        row["telemetry_reconciliation_status"],
        "hot_loop_observation_snapshot_pending"
    );
    assert_eq!(
        row["observability_failure_class"],
        "live_loop_observability_partial"
    );
    assert!(
        row["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("loop measure --node unit_loop_run_snapshot")
    );
    std::fs::remove_dir_all(root).expect("cleanup loop run snapshot");
}

#[test]
fn loop_run_snapshot_preserves_command_failure_details_without_observation_roundtrip() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-run-snapshot-command-failure",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = product_surface(
        "unit_loop_run_snapshot_failure",
        "printf same-command",
        "printf \"failed_law=loop-law failed_check=loop-check why=narrow command failed where=loop.measure claim_impact=blocks_hot_loop next_repair=repair narrow command receipt=validation_artifacts/observability/live-loop-node-timing.json run_id=run-snapshot correlation_id=corr-snapshot query_logs='observe logs query' query_metrics='observe metrics query' query_traces='observe traces query'\"; exit 2",
    );

    let row = measure_surface(
        &root,
        &command_for("hot", "verified-local"),
        surface,
        "sha256:current",
        &inputs(),
        "changed_files_digest_bound",
        ObservationMode::LoopRunSnapshot,
    );

    assert_eq!(row["validation_status"], "fail");
    assert_eq!(row["observability_status"], "partial");
    assert_eq!(
        row["verified_local_failure"]["why_failed"],
        "narrow command failed"
    );
    assert_eq!(
        row["verified_local_failure"]["where_failed"],
        "loop.measure"
    );
    assert_eq!(
        row["verified_local_failure"]["next_repair"],
        "repair narrow command"
    );
    std::fs::remove_dir_all(root).expect("cleanup loop run snapshot failure");
}
