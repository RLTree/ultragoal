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
        failure: Default::default(),
    }
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
