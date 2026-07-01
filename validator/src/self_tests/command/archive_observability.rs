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
    let root = crate::self_tests::boundaries::support::temp_root(label);
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
    let zip = root
        .join("receipts/candidate.zip")
        .to_string_lossy()
        .to_string();
    let receipt = root
        .join("receipts/archive.json")
        .to_string_lossy()
        .to_string();
    let observability = "observability/archive-build.json";
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "archive",
            "build",
            "--zip",
            &zip,
            "--receipt",
            &receipt,
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
