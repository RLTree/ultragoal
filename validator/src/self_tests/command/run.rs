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

#[test]
fn command_dispatch_builds_archive_receipt() {
    let root = package_root("command-dispatch-success");
    let archive_receipt = "validation_artifacts/review/archive.json";
    let archive_zip = "validation_artifacts/review/candidate.zip";
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "archive",
            "build",
            "--zip",
            archive_zip,
            "--receipt",
            archive_receipt,
            "--zip-root",
            "harness-ultragoal",
        ],
    ))
    .expect("archive command");
    assert_eq!(code, 0);
    assert!(root.join(archive_zip).is_file());
    let archive =
        crate::json_boundary::read_json(&root.join(archive_receipt)).expect("archive receipt json");
    assert_eq!(archive["status"], "pass");

    std::fs::remove_dir_all(root).expect("cleanup command dispatch");
}

#[test]
fn command_run_returns_exit_codes_without_exiting_test_process() {
    let root = package_root("command-run-exit-code");
    let update_goal_receipt =
        std::path::PathBuf::from("validation_artifacts/cli/update-goal-eligibility.json");
    let code = crate::command_run::run(args(
        root.clone(),
        &[
            "update-goal",
            "eligibility",
            "--receipt",
            update_goal_receipt
                .to_str()
                .expect("update goal receipt path"),
        ],
    ))
    .expect("control command returns code");
    assert_eq!(code, 1);
    assert!(root.join(&update_goal_receipt).is_file());
    let code = crate::command_run::run(args(root.clone(), &["package", "digest"]))
        .expect("package digest returns code");
    assert_eq!(code, 0);
    let code = crate::command_run::run(args(root.clone(), &["help"])).expect("help returns code");
    assert_eq!(code, 0);
    std::fs::remove_dir_all(root).expect("cleanup command run exit code");
}

#[test]
fn command_run_propagates_package_and_packet_builder_errors() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("command-run-errors");
    let code = crate::command_run::run_with_exit_code(args(root.clone(), &["package", "digest"]))
        .expect("missing manifest returns package digest fail code");
    assert_eq!(code, 1);
    assert!(
        !root.exists(),
        "read-only package digest failure must not create its package root or a receipt"
    );
    let error = crate::package::inventory::package_digest(&root)
        .expect_err("missing manifest remains a stable failure");
    assert!(error.contains("plugin-manifest-draft.json"), "{error}");
    assert!(error.contains("package manifest unavailable"), "{error}");

    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["../escape.txt"]}),
    );
    std::fs::remove_dir_all(root).expect("cleanup command run errors");
}
