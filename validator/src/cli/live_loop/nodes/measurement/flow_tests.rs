use super::*;
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use crate::scheduler::TaskClass;
use std::path::PathBuf;

fn command() -> LiveLoopCommand {
    LiveLoopCommand {
        action: crate::cli::live_loop::LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: PathBuf::from("validation_artifacts/observability/live-loop-node-timing.json"),
        node_id: Some("unit_measure_branch".to_string()),
        measure_all: false,
    }
}

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
        command: "live loop measurement branch contract",
        canonical_full_command,
        narrow_rerun,
        telemetry_reconciliation_state: "requires_command_telemetry_roundtrip",
        execution_task_class: TaskClass::PureReadParallel,
        execution_serial_reason: "none",
        high_frequency: true,
        hot_loop_policy: "routine_hot_repair",
    }
}

#[test]
fn measurement_executes_separate_baseline_when_canonical_command_differs_from_narrow_rerun() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-measure-separate-baseline",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = product_surface(
        "unit_measure_branch",
        "printf baseline-product-behavior",
        "printf verified-product-behavior",
    );
    let inputs = ChangedInputs::for_tests(
        &crate::digest::bytes(b"changed"),
        &crate::digest::bytes(b"audit"),
    );
    let command = command();
    let replay_store = cache_replay::ReplayStore::load(&root, &command);

    let row = measure_surface(
        &root,
        &command,
        surface,
        "sha256:current",
        &inputs,
        &replay_store,
        "changed_files_digest_bound",
        ObservationMode::FullRoundtrip,
    );

    assert_eq!(row["validation_status"], "pass");
    assert_eq!(row["validation_cache_status"], "reusable");
    assert!(matches!(
        row["observability_status"].as_str(),
        Some("partial" | "unavailable")
    ));
    assert_eq!(row["speed_claim_status"], "withheld");
    assert!(
        row["telemetry_reconciliation_status"]
            .as_str()
            .is_some_and(|status| !status.is_empty() && status != "missing")
    );
    assert_eq!(row["baseline_proof_kind"], "executed");
    assert_eq!(
        row["baseline_invalidation_proof"],
        "baseline_command_executed_for_current_measurement"
    );
    std::fs::remove_dir_all(root).expect("cleanup separate baseline");
}

#[test]
fn measurement_reuses_executed_narrow_command_when_baseline_command_is_identical() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-measure-identical-baseline",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = product_surface(
        "unit_same_command",
        "printf same-command",
        "printf same-command",
    );
    let inputs = ChangedInputs::for_tests(
        &crate::digest::bytes(b"changed"),
        &crate::digest::bytes(b"audit"),
    );
    let command = command();
    let replay_store = cache_replay::ReplayStore::load(&root, &command);

    let row = measure_surface(
        &root,
        &command,
        surface,
        "sha256:current",
        &inputs,
        &replay_store,
        "changed_files_digest_bound",
        ObservationMode::FullRoundtrip,
    );

    assert_eq!(row["baseline_proof_kind"], "executed_same_command_reuse");
    assert_eq!(
        row["baseline_invalidation_proof"],
        "baseline_reused_from_executed_narrow_command_because_canonical_full_command_matches"
    );
    assert_eq!(row["validation_status"], "pass");
    std::fs::remove_dir_all(root).expect("cleanup identical baseline");
}

#[test]
fn hot_measurement_reconciles_observability_without_erasing_validation_result() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-measure-observability-reconciliation",
    );
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &serde_json::json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let surface = product_surface(
        "unit_reconciled_command",
        "printf baseline-product-behavior",
        "printf verified-product-behavior",
    );
    let inputs = ChangedInputs::for_tests(
        &crate::digest::bytes(b"changed"),
        &crate::digest::bytes(b"audit"),
    );
    let command = command_for("hot", "verified-local");
    let replay_store = cache_replay::ReplayStore::load(&root, &command);

    let row = measure_surface(
        &root,
        &command,
        surface,
        &crate::package::inventory::package_digest(&root).expect("candidate"),
        &inputs,
        &replay_store,
        "changed_files_digest_bound",
        ObservationMode::FullRoundtrip,
    );

    assert_eq!(row["validation_status"], "pass");
    assert_eq!(row["validation_cache_status"], "reusable");
    assert!(
        row["telemetry_reconciliation_status"]
            .as_str()
            .is_some_and(|status| !status.is_empty() && status != "missing")
    );
    assert_ne!(row["observability_status"], "unavailable");
    assert_ne!(row["where_failed"], "");
    std::fs::remove_dir_all(root).expect("cleanup observability reconciliation");
}
