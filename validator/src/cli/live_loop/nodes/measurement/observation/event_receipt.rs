use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;
use std::path::{Path, PathBuf};

fn measure_command() -> LiveLoopCommand {
    LiveLoopCommand {
        action: crate::cli::live_loop::LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: PathBuf::from("validation_artifacts/observability/live-loop-node-timing.json"),
        node_id: Some("changed_files".to_string()),
        measure_all: false,
    }
}

fn command_run(duration_ms: u64) -> FullCommandRun {
    FullCommandRun {
        exit_code: 0,
        status_success: true,
        launch_error: false,
        duration_ms,
        stdout_digest: "sha256:stdout".to_string(),
        stderr_digest: "sha256:stderr".to_string(),
        executed_test_count: None,
        failure: Default::default(),
    }
}

fn command_run_with_status(exit_code: i32, status_success: bool) -> FullCommandRun {
    let mut run = command_run(23);
    run.exit_code = exit_code;
    run.status_success = status_success;
    run
}

fn write_minimal_manifest(root: &Path) -> String {
    std::fs::create_dir_all(root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    crate::package::inventory::package_digest(root).expect("candidate")
}

#[test]
fn command_observation_receipt_names_actual_node_command_surface() {
    let root = temp_root("live-loop-command-observation");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt = event::receipt_path(surface.id);
    let observation = event::write(
        &root,
        surface,
        &candidate,
        &measure_command(),
        &command_run(7),
        &receipt,
    )
    .expect("command observation");

    assert_eq!(observation.value["status"], "pass");
    assert_eq!(
        observation.value["event"]["operation"],
        "loop.measure.changed_files"
    );
    assert_eq!(observation.value["event"]["surface"], "candidate_delta");
    assert_eq!(observation.value["event"]["duration_ms"], 7);
    assert_eq!(
        observation.value["event"]["claim_impact"],
        "source_local_live_loop_node_observation_only_not_speed_claim"
    );
    assert!(root.join(receipt).is_file());
    std::fs::remove_dir_all(root).expect("cleanup command observation");
}

#[test]
fn command_observation_records_failed_verified_work_without_speed_claim() {
    let root = temp_root("live-loop-command-observation-fail");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt = event::receipt_path(surface.id);
    let value = event::write(
        &root,
        surface,
        &candidate,
        &measure_command(),
        &command_run_with_status(2, false),
        &receipt,
    )
    .expect("failed command observation");

    assert_eq!(value.value["status"], "fail");
    assert_eq!(
        value.value["event"]["failure_class"],
        "verified_local_command_failed"
    );
    assert_eq!(
        value.value["event"]["claim_impact"],
        "source_local_live_loop_node_observation_only_not_speed_claim"
    );
    std::fs::remove_dir_all(root).expect("cleanup failed command observation");
}

#[test]
fn command_observation_rejects_external_receipt_path() {
    let root = temp_root("live-loop-command-observation-path");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    let err = event::write(
        &root,
        surface,
        &candidate,
        &measure_command(),
        &command_run_with_status(0, true),
        Path::new("/tmp/live-loop-command-observation.json"),
    )
    .expect_err("absolute observation receipt rejected");

    assert!(err.contains("external debug only") || err.contains("outside root"));
    std::fs::remove_dir_all(root).expect("cleanup rejected command observation");
}

#[test]
fn command_observation_uses_product_identity_for_ultragoal_surface() {
    let root = temp_root("live-loop-command-observation-product-identity");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("package_digest").expect("surface");
    let receipt = event::receipt_path(surface.id);
    let value = event::write(
        &root,
        surface,
        &candidate,
        &measure_command(),
        &command_run_with_status(0, true),
        &receipt,
    )
    .expect("ultragoal command observation");

    assert_eq!(value.value["event"]["command"], "ultragoal");
    assert!(
        !value.value["event"]["command"]
            .as_str()
            .expect("command")
            .contains('/')
    );
    assert_eq!(
        value.value["event"]["subcommand"],
        "--root . package digest"
    );
    std::fs::remove_dir_all(root).expect("cleanup product identity observation");
}

#[test]
fn command_observation_preserves_program_identity_when_path_has_no_file_name() {
    let root = temp_root("live-loop-command-observation-program-identity");
    let candidate = write_minimal_manifest(&root);
    let surface = crate::cli::live_loop::surfaces::LoopValidationSurface {
        id: "unit_program_identity",
        surface: "command_observation",
        command: "command observation product identity",
        canonical_full_command: "/",
        narrow_rerun: "/",
        telemetry_reconciliation_state: "requires_command_telemetry_roundtrip",
        execution_task_class: crate::scheduler::TaskClass::PureReadParallel,
        execution_serial_reason: "none",
        high_frequency: true,
        hot_loop_policy: "routine_hot_repair",
    };
    let receipt = event::receipt_path(surface.id);

    let value = event::write(
        &root,
        surface,
        &candidate,
        &measure_command(),
        &command_run_with_status(0, true),
        &receipt,
    )
    .expect("path command observation");

    assert_eq!(value.value["event"]["command"], "/");
    std::fs::remove_dir_all(root).expect("cleanup path identity observation");
}

#[test]
fn reconciliation_reports_command_observation_write_failure() {
    let root = temp_root("live-loop-command-observation-write-failure");
    std::fs::create_dir_all(&root).expect("root");
    let root_file = root.join("not-a-directory");
    std::fs::write(&root_file, "not a directory").expect("root file");
    let surface = surface_by_id("changed_files").expect("surface");

    let reconciliation = reconcile(
        &root_file,
        surface,
        "sha256:candidate",
        &measure_command(),
        &command_run(7),
    );

    assert_eq!(reconciliation.status, "command_observation_failed");
    assert_eq!(
        reconciliation.value["claim_impact"],
        "live_loop_node_timing_blocked"
    );
    assert!(
        reconciliation.value["failure"]
            .as_str()
            .unwrap()
            .contains("Not a directory")
    );
    std::fs::remove_dir_all(root).expect("cleanup command observation write failure");
}

#[test]
fn command_observation_field_parser_names_missing_required_field() {
    let err = event::CommandObservation::from_receipt(json!({"run_id":"run-present"}))
        .expect_err("missing trace id rejected");

    assert_eq!(err, "live-loop command observation missing correlation_id");
}

#[test]
fn command_observation_identity_parser_requires_trace_id() {
    let err = event::CommandObservation::from_receipt(json!({
        "run_id": "run-present",
        "correlation_id": "corr-present"
    }))
    .expect_err("missing trace id rejected");

    assert_eq!(err, "live-loop command observation missing trace_id");
}
