use super::{LiveLoopAction, LiveLoopCommand, run};
use serde_json::json;

#[test]
fn live_loop_run_writes_source_local_blocker_receipt() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-run");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-test.json".into(),
        node_id: None,
        measure_all: false,
    };
    let code = run(&root, &command).expect("loop run");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(&command.receipt)).expect("receipt");
    assert_eq!(receipt["schema"], "harness-ultragoal.loop-run-receipt.v1");
    assert_eq!(receipt["cache_mode"], "verified-local");
    assert_eq!(receipt["claim_ceiling"], "source-local loop proof only");
    assert_eq!(
        receipt["audit_context"]["package_truth"]["snapshot_authority"],
        "AuditContext.package_truth_snapshot"
    );
    assert_eq!(
        receipt["audit_context"]["package_truth"]["claim_limit"],
        "source_package_truth_only_not_install_cache_registry_or_readiness"
    );
    assert!(receipt["worker_count"].as_u64().unwrap() >= 1);
}

#[test]
fn live_loop_measure_and_format_actions_delegate_to_product_commands() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-action-delegation");
    std::fs::create_dir_all(&root).expect("root");
    let measure_command = LiveLoopCommand {
        action: LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-measure.json".into(),
        node_id: Some("not-a-current-node".to_string()),
        measure_all: false,
    };
    let err = run(&root, &measure_command).expect_err("unknown measure node");
    assert!(
        err.contains("unknown live-loop node: not-a-current-node"),
        "{err}"
    );

    let format_command = LiveLoopCommand {
        action: LiveLoopAction::FormatCheck,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: "validation_artifacts/observability/loop-format-check.json".into(),
        node_id: None,
        measure_all: false,
    };
    let code = run(&root, &format_command).expect("format action delegates to routine formatter");
    assert_eq!(code, 1);
    std::fs::remove_dir_all(root).expect("cleanup action delegation root");
}

#[test]
fn live_loop_run_fails_closed_before_work_for_bad_roots_and_jobs() {
    let missing_manifest =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-missing");
    std::fs::create_dir_all(&missing_manifest).expect("root");
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-test.json".into(),
        node_id: None,
        measure_all: false,
    };
    let err = run(&missing_manifest, &command).expect_err("missing manifest");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");

    let invalid_jobs =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-jobs");
    std::fs::create_dir_all(&invalid_jobs).expect("root");
    crate::json_boundary::write_json(
        &invalid_jobs.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let bad_jobs = LiveLoopCommand {
        jobs: Some(0),
        ..command
    };
    let err = run(&invalid_jobs, &bad_jobs).expect_err("zero jobs");
    assert!(err.contains("scheduler jobs must be at least 1"), "{err}");
}

#[test]
fn live_loop_run_fails_closed_when_authority_receipts_cannot_be_written() {
    let current_state_root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-current-state-write",
    );
    std::fs::create_dir_all(&current_state_root).expect("root");
    crate::json_boundary::write_json(
        &current_state_root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    std::fs::write(current_state_root.join("validation_artifacts"), b"file").expect("block dir");
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-test.json".into(),
        node_id: None,
        measure_all: false,
    };
    let err = run(&current_state_root, &command).expect_err("current-state write fails");
    assert!(err.contains("validation_artifacts"), "{err}");

    let receipt_root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-receipt-root");
    std::fs::create_dir_all(&receipt_root).expect("root");
    crate::json_boundary::write_json(
        &receipt_root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let bad_receipt = LiveLoopCommand {
        receipt: "artifacts/loop-test.json".into(),
        ..command
    };
    let err = run(&receipt_root, &bad_receipt).expect_err("ungoverned receipt root rejected");
    assert!(err.contains("governed claim artifact root"), "{err}");
    assert!(err.contains("external debug only"), "{err}");

    let observability_root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-observability-write",
    );
    std::fs::create_dir_all(observability_root.join("validation_artifacts")).expect("root");
    crate::json_boundary::write_json(
        &observability_root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    std::fs::write(
        observability_root.join("validation_artifacts/observability"),
        b"file",
    )
    .expect("block observability dir");
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/loop-receipt.json".into(),
        node_id: None,
        measure_all: false,
    };
    let err = run(&observability_root, &command).expect_err("observability spool write fails");
    assert!(err.contains("validation_artifacts/observability"), "{err}");
}

#[test]
fn live_loop_run_fails_closed_when_current_state_or_receipt_file_targets_are_blocked() {
    let current_state_root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-current-state-file-target",
    );
    std::fs::create_dir_all(current_state_root.join("validation_artifacts/current-state.json"))
        .expect("block current-state file target");
    crate::json_boundary::write_json(
        &current_state_root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/loop-receipt.json".into(),
        node_id: None,
        measure_all: false,
    };
    let err = run(&current_state_root, &command).expect_err("current-state file write fails");
    assert!(
        err.contains("validation_artifacts/current-state.json"),
        "{err}"
    );
    assert!(err.contains("json"), "{err}");

    let receipt_parent_root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-receipt-parent-file",
    );
    std::fs::create_dir_all(receipt_parent_root.join("validation_artifacts")).expect("root");
    crate::json_boundary::write_json(
        &receipt_parent_root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    std::fs::write(
        receipt_parent_root.join("validation_artifacts/observability"),
        b"file",
    )
    .expect("block receipt parent");
    let command = LiveLoopCommand {
        receipt: "validation_artifacts/observability/loop-test.json".into(),
        ..command
    };
    let err = run(&receipt_parent_root, &command).expect_err("receipt parent prepare fails");
    assert!(err.contains("validation_artifacts/observability"), "{err}");
    assert!(err.contains("create parent failed"), "{err}");
}

#[test]
fn live_loop_run_rejects_absolute_claim_artifact_receipts() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-absolute-receipt");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "/tmp/ultragoal-loop-receipt.json".into(),
        node_id: None,
        measure_all: false,
    };
    let err = run(&root, &command).expect_err("absolute receipt rejected");
    assert!(err.contains("root-relative claim artifact path"), "{err}");
    assert!(err.contains("external debug only"), "{err}");
}
