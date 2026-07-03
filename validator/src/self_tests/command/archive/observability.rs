use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

fn package_root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/file.txt"), "package payload\n").expect("file");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["docs/file.txt"]}),
    );
    root
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or("")
}

#[test]
fn archive_command_emits_pass_observability() {
    let root = package_root("archive-observability-pass");
    let zip = "receipts/candidate.zip";
    let receipt = "receipts/archive.json";
    let observability = "observability/archive-build.json";
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "archive",
            "build",
            "--zip",
            zip,
            "--receipt",
            receipt,
            "--observability-receipt",
            observability,
            "--zip-root",
            "harness-ultragoal",
        ],
    ))
    .expect("archive command");
    assert_eq!(code, 0);
    let obs = crate::json_boundary::read_json(&root.join(observability)).expect("observability");
    assert_eq!(text(&obs, "status"), "pass");
    assert_eq!(text(&obs, "operation"), "archive.build");
    assert_eq!(
        text(&obs, "check_id"),
        "archive-build-observability-binding"
    );
    assert_eq!(text(&obs, "claim_id"), "archive_source_local_observability");
    assert_eq!(obs["event"]["task_count"], 3);
    assert_eq!(obs["event"]["queue_depth"], 3);
    assert_eq!(obs["event"]["cache_mode"], "archive_digest_no_cache");
    assert!(root.join("receipts/candidate.zip").is_file());
    assert!(root.join("receipts/archive.json").is_file());
    std::fs::remove_dir_all(root).expect("cleanup archive pass");
}

#[test]
fn archive_command_emits_fail_closed_observability() {
    let root = package_root("archive-observability-fail");
    let observability = "observability/archive-build-fail.json";
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "archive",
            "build",
            "--zip",
            "receipts/candidate.zip",
            "--receipt",
            "receipts/archive.json",
            "--observability-receipt",
            observability,
            "--archive-purpose",
            "upload-ready",
        ],
    ))
    .expect("archive command returns fail code");
    assert_eq!(code, 1);
    let obs = crate::json_boundary::read_json(&root.join(observability)).expect("observability");
    assert_eq!(text(&obs, "status"), "fail");
    assert_eq!(text(&obs, "failure_class"), "archive_build_failure");
    assert!(text(&obs, "why_failed").contains("archive purpose must be candidate_review_anchor"));
    assert!(
        obs["blocked_claims"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim == "update_goal_eligibility")
    );
    std::fs::remove_dir_all(root).expect("cleanup archive fail");
}

#[test]
fn archive_observability_receipt_path_validation_is_fail_closed() {
    for raw in [
        vec!["--observability-receipt", ""],
        vec!["--observability-receipt", "../outside.json"],
    ] {
        let args = raw.iter().map(|item| item.to_string()).collect::<Vec<_>>();
        let error = crate::cli::archive::observability_receipt(&args)
            .expect_err("unsafe observability receipt rejected");
        assert!(error.contains("observability receipt"), "{error}");
    }
}

#[test]
fn archive_run_rejects_absolute_observability_claim_artifact() {
    let root = package_root("archive-observability-absolute-run");
    let error = crate::cli::archive::run(
        root.clone(),
        PathBuf::from("receipts/candidate.zip"),
        PathBuf::from("receipts/archive.json"),
        root.join("absolute-archive-observability.json"),
        "harness-ultragoal".to_string(),
        "candidate_review_anchor".to_string(),
    )
    .expect_err("absolute observability receipt rejected by runner");
    assert!(error.contains("archive observability receipt"), "{error}");
    assert!(
        error.contains("root-relative claim artifact path"),
        "{error}"
    );
    assert!(error.contains("external debug only"), "{error}");
    std::fs::remove_dir_all(root).expect("cleanup archive absolute run");
}

#[test]
fn archive_command_records_archive_receipt_write_failures() {
    let root = package_root("archive-observability-receipt-write-fail");
    let receipt_dir = root.join("receipts/archive.json");
    std::fs::create_dir_all(&receipt_dir).expect("receipt dir blocks file write");
    let observability = "observability/archive-build-write-fail.json";

    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "archive",
            "build",
            "--zip",
            "receipts/candidate.zip",
            "--receipt",
            "receipts/archive.json",
            "--observability-receipt",
            observability,
            "--zip-root",
            "harness-ultragoal",
        ],
    ))
    .expect("archive command returns fail code");

    assert_eq!(code, 1);
    let obs = crate::json_boundary::read_json(&root.join(observability)).expect("observability");
    assert_eq!(text(&obs, "status"), "fail");
    assert!(
        text(&obs, "why_failed").contains("json rename failed"),
        "{obs}"
    );
    assert!(!text(&obs, "why_failed").contains(&root.to_string_lossy().to_string()));
    std::fs::remove_dir_all(root).expect("cleanup archive write fail");
}

#[test]
fn archive_command_propagates_observability_spool_write_errors() {
    let root = package_root("archive-observability-spool-write-fail");
    std::fs::write(root.join("validation_artifacts"), b"not a directory").expect("block spool");

    let error = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "archive",
            "build",
            "--zip",
            "receipts/candidate.zip",
            "--receipt",
            "receipts/archive.json",
            "--observability-receipt",
            "observability/archive-build.json",
            "--zip-root",
            "harness-ultragoal",
        ],
    ))
    .expect_err("observability spool failure propagates");

    assert!(
        error.contains("validation_artifacts/observability/spool"),
        "{error}"
    );
    std::fs::remove_dir_all(root).expect("cleanup archive spool fail");
}
