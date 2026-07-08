use super::fixtures::{
    LIVE_LOOP_TIMING_RECEIPT, command, live_loop_timing_receipt_arg, verified_local_proof,
};
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;

#[test]
fn live_loop_measure_rejects_missing_or_unknown_node_id() {
    let root = temp_root("live-loop-measure-errors");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");

    let missing = crate::cli::live_loop::run(&root, &command(None, live_loop_timing_receipt_arg()))
        .expect_err("missing node rejected");
    assert!(
        missing.contains("loop measure requires --node <id>"),
        "{missing}"
    );

    let unknown = crate::cli::live_loop::run(
        &root,
        &command(Some("unknown_node"), live_loop_timing_receipt_arg()),
    )
    .expect_err("unknown node rejected");
    assert!(unknown.contains("unknown live-loop node"), "{unknown}");

    std::fs::remove_dir_all(root).expect("cleanup measure errors");
}

#[test]
fn live_loop_measure_reports_digest_receipt_and_launch_failures() {
    let missing_manifest = temp_root("live-loop-measure-digest");
    std::fs::create_dir_all(&missing_manifest).expect("missing manifest root");
    let digest_err = crate::cli::live_loop::run(
        &missing_manifest,
        &command(Some("changed_files"), live_loop_timing_receipt_arg()),
    )
    .expect_err("missing manifest rejected");
    assert!(
        digest_err.contains("plugin-manifest-draft.json"),
        "{digest_err}"
    );
    std::fs::remove_dir_all(missing_manifest).expect("cleanup missing manifest");

    let root = temp_root("live-loop-measure-receipt");
    std::fs::create_dir_all(&root).expect("receipt root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let receipt_err = crate::cli::live_loop::run(
        &root,
        &command(Some("changed_files"), root.join(LIVE_LOOP_TIMING_RECEIPT)),
    )
    .expect_err("absolute receipt rejected");
    assert!(
        receipt_err.contains("root-relative claim artifact path"),
        "{receipt_err}"
    );

    let surface = crate::cli::live_loop::surfaces::LoopValidationSurface {
        id: "unit_launch_failure",
        surface: "rust_validation",
        command: "launch failure classifier contract",
        canonical_full_command: "/missing/live-loop-command",
        narrow_rerun: "/missing/live-loop-command",
        telemetry_reconciliation_state: "requires_command_telemetry_roundtrip",
        execution_task_class: crate::scheduler::TaskClass::PureReadParallel,
        execution_serial_reason: "none",
        high_frequency: true,
        hot_loop_policy: "routine_hot_repair",
    };
    let launch_failure = super::super::full_command::run_full_command(&root, surface);
    assert!(!launch_failure.status_success);
    assert!(launch_failure.launch_error);
    assert_eq!(
        super::super::timing::failure::measurement_failure_class(
            &launch_failure,
            &verified_local_proof(0, true, 1, "pass"),
            0,
        ),
        "canonical_full_command_launch_failed"
    );

    std::fs::remove_dir_all(root).expect("cleanup receipt root");
}
