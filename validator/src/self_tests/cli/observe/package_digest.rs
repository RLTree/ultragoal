use std::fs;

#[test]
fn package_digest_command_emits_observability_receipt_contract() {
    let root = super::minimal_root("package-digest-observability");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::PackageDigest,
    })
    .expect("package digest command");
    assert_eq!(code, 0);
    let receipt_path = root.join("validation_artifacts/observability/package-digest.json");
    let receipt = crate::json_boundary::read_json(&receipt_path).expect("package digest receipt");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    assert_eq!(
        receipt["schema"],
        crate::cli::observe::types::RECEIPT_SCHEMA
    );
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["candidate_digest"], candidate);
    assert_eq!(receipt["law_id"], crate::cli::observe::types::LAW_ID);
    assert_eq!(receipt["check_id"], "package-digest-observability-binding");
    assert_eq!(receipt["claim_id"], "source_package_digest");
    assert_eq!(receipt["event"]["operation"], "package.digest");
    assert!(receipt["event"]["duration_ms"].as_u64().unwrap() > 0);
    assert_eq!(receipt["event"]["worker_count"], 1);
    assert_eq!(receipt["event"]["task_count"], 1);
    assert_eq!(receipt["event"]["queue_depth"], 0);
    assert_eq!(receipt["event"]["cache_mode"], "package_digest_no_cache");
    assert_eq!(
        receipt["event"]["resource_measurement_status"],
        "wall_time_only_cpu_memory_io_unavailable"
    );
    assert_eq!(
        receipt["metric"]["duration_ms"],
        receipt["event"]["duration_ms"]
    );
    assert_eq!(
        receipt["trace"]["worker_count"],
        receipt["event"]["worker_count"]
    );
    let child_spans = receipt["trace"]["child_spans"]
        .as_array()
        .expect("trace child spans");
    assert!(
        child_spans
            .iter()
            .any(|span| span["span_kind"] == "validator_check")
    );
    assert!(child_spans.iter().all(|span| {
        span["parent_span_id"] == receipt["trace"]["span_id"]
            && span["trace_id"] == receipt["trace"]["trace_id"]
    }));
    assert!(
        receipt["supported_claims"]
            .as_array()
            .expect("supported")
            .iter()
            .any(|item| item.as_str() == Some("source_package_digest"))
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|item| item.as_str() == Some("update_goal_eligibility"))
    );
    fs::remove_dir_all(root).expect("cleanup package digest observability");
}

#[test]
fn package_digest_command_emits_fail_stdout_and_receipt_contract() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("package-digest-fail");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::PackageDigest,
    })
    .expect("package digest command returns fail code");
    assert_eq!(code, 1);
    let receipt_path = root.join("validation_artifacts/observability/package-digest.json");
    let receipt = crate::json_boundary::read_json(&receipt_path).expect("package digest receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["candidate_digest"], crate::digest::ZERO);
    assert_eq!(receipt["failure_class"], "package_digest_failure");
    assert_eq!(receipt["where_failed"], "package.digest");
    assert!(
        receipt["why_failed"]
            .as_str()
            .unwrap()
            .contains("json read failed")
    );
    let stdout = crate::cli::package::digest::stdout_contract_for_test(&receipt);
    assert_eq!(stdout.len(), 2);
    assert!(stdout[0].contains("ultragoal-package-digest fail"));
    assert!(stdout[0].contains("proven=none"));
    assert!(stdout[0].contains("supported_claims=none"));
    assert!(stdout[0].contains("unsupported_claims="));
    assert!(stdout[1].contains("failed_check=package-digest-observability-binding"));
    assert!(stdout[1].contains("next_repair="));
    fs::remove_dir_all(root).expect("cleanup package digest fail");
}

#[test]
fn package_digest_command_propagates_observability_write_failures() {
    let root = super::minimal_root("package-digest-write-fail");
    fs::write(root.join("validation_artifacts"), "not a directory").expect("block artifacts dir");
    let err = crate::cli::package::digest::run(&root).expect_err("receipt write should fail");
    assert!(
        err.contains("validation_artifacts") || err.contains("Not a directory"),
        "{err}"
    );
    fs::remove_dir_all(root).expect("cleanup package digest write fail");
}
