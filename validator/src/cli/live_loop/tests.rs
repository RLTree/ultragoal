use super::{LiveLoopCommand, parse, run};
use serde_json::json;

#[test]
fn parser_accepts_auto_jobs_and_rejects_invalid_jobs() {
    let command = parse(
        &[
            "loop",
            "run",
            "--tier",
            "hot",
            "--cache-mode",
            "verified-local",
            "--jobs",
            "auto",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
    )
    .expect("parse")
    .expect("loop command");
    assert_eq!(command.tier, "hot");
    assert_eq!(command.cache_mode, "verified-local");
    assert_eq!(command.jobs, None);

    let err = parse(
        &["loop", "run", "--jobs", "many"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("invalid jobs");
    assert!(err.contains("invalid --jobs value"));
}

#[test]
fn live_loop_run_writes_source_local_blocker_receipt() {
    let root = crate::self_tests::boundaries::support::temp_root("live-loop-run");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let command = LiveLoopCommand {
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-test.json".into(),
    };
    let code = run(&root, &command).expect("loop run");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(&command.receipt)).expect("receipt");
    assert_eq!(receipt["schema"], "harness-ultragoal.loop-run-receipt.v1");
    assert_eq!(receipt["cache_mode"], "verified-local");
    assert_eq!(receipt["claim_ceiling"], "source-local loop proof only");
    assert!(receipt["worker_count"].as_u64().unwrap() >= 1);
}

#[test]
fn live_loop_run_fails_closed_before_work_for_bad_roots_and_jobs() {
    let missing_manifest = crate::self_tests::boundaries::support::temp_root("live-loop-missing");
    std::fs::create_dir_all(&missing_manifest).expect("root");
    let command = LiveLoopCommand {
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-test.json".into(),
    };
    let err = run(&missing_manifest, &command).expect_err("missing manifest");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");

    let invalid_jobs = crate::self_tests::boundaries::support::temp_root("live-loop-jobs");
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
    let current_state_root =
        crate::self_tests::boundaries::support::temp_root("live-loop-current-state-write");
    std::fs::create_dir_all(&current_state_root).expect("root");
    crate::json_boundary::write_json(
        &current_state_root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    std::fs::write(current_state_root.join("validation_artifacts"), b"file").expect("block dir");
    let command = LiveLoopCommand {
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-test.json".into(),
    };
    let err = run(&current_state_root, &command).expect_err("current-state write fails");
    assert!(err.contains("validation_artifacts"), "{err}");

    let receipt_root = crate::self_tests::boundaries::support::temp_root("live-loop-receipt-write");
    std::fs::create_dir_all(&receipt_root).expect("root");
    crate::json_boundary::write_json(
        &receipt_root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let bad_receipt = LiveLoopCommand {
        receipt: "plugin-manifest-draft.json/loop-test.json".into(),
        ..command
    };
    let err = run(&receipt_root, &bad_receipt).expect_err("loop receipt write fails");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");

    let observability_root =
        crate::self_tests::boundaries::support::temp_root("live-loop-observability-write");
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
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/loop-receipt.json".into(),
    };
    let err = run(&observability_root, &command).expect_err("observability spool write fails");
    assert!(err.contains("validation_artifacts/observability"), "{err}");
}

#[test]
fn live_loop_run_can_pass_when_current_state_has_no_blocker() {
    let root = crate::self_tests::boundaries::support::temp_root("live-loop-pass");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({"observability_control_board": {"status": "observable"}}),
    )
    .expect("board");
    for path in [
        "validation_artifacts/coverage/coverage-receipt.json",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "validation_artifacts/ultragoal-audit/red-fixture-report.json",
    ] {
        crate::json_boundary::write_json(
            &root.join(path),
            &json!({"status": "pass", "target_digest": candidate}),
        )
        .expect("source-local receipt");
    }

    let command = LiveLoopCommand {
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-pass.json".into(),
    };
    let code = run(&root, &command).expect("loop pass run");
    assert_eq!(code, 0);
    let receipt = crate::json_boundary::read_json(&root.join(&command.receipt)).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["first_blocker"]["id"], "none");
    assert_eq!(receipt["observability"]["event"]["failure_class"], "none");
    assert_eq!(receipt["observability"]["event"]["where_failed"], "none");
    std::fs::remove_dir_all(root).expect("cleanup live loop pass");
}
