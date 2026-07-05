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
fn command_dispatch_builds_review_archive_and_semantic_receipts() {
    let root = package_root("command-dispatch-success");
    let review_receipt = "validation_artifacts/review/review-target.json";
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &["review-target", "build", "--receipt", review_receipt],
    ))
    .expect("review-target command");
    assert_eq!(code, 0);
    let review =
        crate::json_boundary::read_json(&root.join(review_receipt)).expect("review receipt json");
    assert_eq!(review["status"], "pass");

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

    write_json(
        &root.join("claims.json"),
        &json!({"claims":[{"id":"CLAIM/ONE","title":"Review ready","description":"External live surface claim"}]}),
    );
    let semantic_dir = root.join("semantic");
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "semantic-receipts",
            "--input",
            "claims.json",
            "--out-dir",
            semantic_dir.to_str().expect("semantic dir"),
            "--producer-actor-id",
            "author",
            "--classifier-actor-id",
            "classifier",
        ],
    ))
    .expect("semantic command");
    assert_eq!(code, 0);
    let semantic = crate::json_boundary::read_json(
        &semantic_dir.join("CLAIM-ONE.semantic-classification-receipt.json"),
    )
    .expect("semantic receipt");
    assert_eq!(semantic["claim_id"], "CLAIM/ONE");
    assert_eq!(semantic["actor_disjoint"], true);
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
    let code = crate::command_run::run(args(root.clone(), &["package-digest"]))
        .expect("package digest returns code");
    assert_eq!(code, 0);
    let code = crate::command_run::run(args(root.clone(), &["help"])).expect("help returns code");
    assert_eq!(code, 0);
    std::fs::remove_dir_all(root).expect("cleanup command run exit code");
}

#[test]
fn command_run_propagates_package_and_packet_builder_errors() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("command-run-errors");
    let code = crate::command_run::run_with_exit_code(args(root.clone(), &["package-digest"]))
        .expect("missing manifest returns package digest fail code");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/package-digest.json"),
    )
    .expect("package digest fail receipt");
    assert_eq!(receipt["status"], "fail");
    let why_failed = receipt["why_failed"].as_str().unwrap();
    assert!(why_failed.contains("json read failed"));

    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["../escape.txt"]}),
    );
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "review-target",
            "build",
            "--receipt",
            "validation_artifacts/review/review-target.json",
        ],
    ))
    .expect("invalid review target returns fail code");
    assert_eq!(code, 1);
    std::fs::remove_dir_all(root).expect("cleanup command run errors");
}

#[test]
fn command_run_propagates_output_and_generation_errors() {
    let review_root = package_root("command-run-review-output-error");
    let blocked = review_root.join("validation_artifacts/review/blocked-parent");
    std::fs::create_dir_all(review_root.join("validation_artifacts/review"))
        .expect("review artifact dir");
    std::fs::write(&blocked, "not a directory").expect("blocked parent file");
    let code = crate::command_run::run_with_exit_code(args(
        review_root.clone(),
        &[
            "review-target",
            "build",
            "--receipt",
            "validation_artifacts/review/blocked-parent/review-target.json",
        ],
    ))
    .expect("review-target receipt output returns fail code");
    assert_eq!(code, 1);
    std::fs::remove_dir_all(review_root).expect("cleanup review output error");

    let semantic_root = package_root("command-run-semantic-errors");
    write_json(
        &semantic_root.join("claims.json"),
        &json!({"claims":[{"id":"CLAIM/MODEL","title":"Model-backed semantic claim"}]}),
    );
    let err = crate::command_run::run_with_exit_code(args(
        semantic_root.clone(),
        &[
            "semantic-receipts",
            "--input",
            "claims.json",
            "--out-dir",
            semantic_root
                .join("semantic")
                .to_str()
                .expect("semantic dir"),
            "--implementation-kind",
            "model",
            "--provider",
            "example-provider",
            "--model",
            "example-model",
        ],
    ))
    .expect_err("semantic generation failure propagates");
    assert!(
        err.contains("require external model or reviewer proof"),
        "{err}"
    );
    std::fs::remove_dir_all(semantic_root).expect("cleanup semantic errors");
}
