use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repo parent")
        .to_path_buf()
}

fn temp_root(root: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    root.join("target")
        .join(format!("ultragoal-cli-surface-{stamp}"))
}

fn validator() -> &'static str {
    env!("CARGO_BIN_EXE_ultragoal-validator")
}

fn ultragoal() -> &'static str {
    env!("CARGO_BIN_EXE_ultragoal")
}

fn run(root: &Path, args: &[String]) -> std::process::Output {
    run_bin(validator(), root, args)
}

fn run_ultragoal(root: &Path, args: &[String]) -> std::process::Output {
    run_bin(ultragoal(), root, args)
}

fn run_bin(bin: &str, root: &Path, args: &[String]) -> std::process::Output {
    Command::new(bin)
        .current_dir(root)
        .args(args)
        .output()
        .expect("validator command runs")
}

#[test]
fn cli_surface_commands_execute() {
    let root = repo_root();
    let temp = temp_root(&root);
    std::fs::create_dir_all(&temp).expect("create temp root");

    for args in [Vec::<String>::new(), vec!["unknown".into()]] {
        assert_eq!(run(&root, &args).status.code(), Some(2));
    }
    assert_eq!(run(&root, &["--root".into()]).status.code(), Some(2));
    assert_eq!(
        run(&root, &["--root".into(), "audit".into()]).status.code(),
        Some(2)
    );
    assert_eq!(run(&root, &["review-target".into()]).status.code(), Some(2));

    let digest = run(
        &root,
        &["--root".into(), ".".into(), "package-digest".into()],
    );
    assert!(digest.status.success(), "package-digest failed: {digest:?}");

    let canonical_digest = run_ultragoal(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "package".into(),
            "digest".into(),
        ],
    );
    assert!(
        canonical_digest.status.success(),
        "canonical package digest failed: {canonical_digest:?}"
    );

    let update_goal_receipt = temp.join("update-goal-eligibility.json");
    let update_goal = run_ultragoal(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "update-goal".into(),
            "eligibility".into(),
            "--receipt".into(),
            update_goal_receipt.display().to_string(),
        ],
    );
    assert_eq!(update_goal.status.code(), Some(1));
    assert_fail_closed_cli_receipt(&update_goal_receipt, "update_goal_eligibility");

    let self_receipt = temp.join("self-law-receipt.json");
    let self_law = run_ultragoal(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "self".into(),
            "update-goal".into(),
            "eligibility".into(),
            "--receipt".into(),
            self_receipt.display().to_string(),
        ],
    );
    assert_eq!(self_law.status.code(), Some(1));
    assert_fail_closed_cli_receipt(&self_receipt, "self_update_goal_eligibility");

    let review_target = temp.join("review-target.json");
    let review_target_args = vec![
        "--root".into(),
        ".".into(),
        "review-target".into(),
        "--receipt".into(),
        review_target.display().to_string(),
    ];
    assert!(run(&root, &review_target_args).status.success());

    let archive_args = vec![
        "--root".into(),
        ".".into(),
        "archive".into(),
        "--zip".into(),
        temp.join("candidate.zip").display().to_string(),
        "--receipt".into(),
        temp.join("archive.json").display().to_string(),
    ];
    assert!(run(&root, &archive_args).status.success());

    let semantic_args = vec![
        "--root".into(),
        ".".into(),
        "semantic-receipts".into(),
        "--input".into(),
        "fixtures/valid/minimal-goal-run.json".into(),
        "--out-dir".into(),
        temp.join("semantic").display().to_string(),
    ];
    assert!(run(&root, &semantic_args).status.success());

    let refused_model = vec![
        "--root".into(),
        ".".into(),
        "semantic-receipts".into(),
        "--input".into(),
        "fixtures/valid/minimal-goal-run.json".into(),
        "--out-dir".into(),
        temp.join("semantic-model-refusal").display().to_string(),
        "--implementation-kind".into(),
        "model".into(),
        "--provider".into(),
        "test-provider".into(),
        "--model".into(),
        "test-model".into(),
        "--prompt-contract-digest".into(),
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    ];
    assert_eq!(run(&root, &refused_model).status.code(), Some(2));

    let red_semantic_args = vec![
        "--root".into(),
        ".".into(),
        "semantic-receipts".into(),
        "--input".into(),
        "fixtures/red/semantic-classification-receipt-wrong-claim.json".into(),
        "--out-dir".into(),
        temp.join("semantic-red").display().to_string(),
    ];
    assert_ne!(run(&root, &red_semantic_args).status.code(), None);

    for (fixture, receipt_name) in [
        (
            "fixtures/target-repo/valid-product-cohesion",
            "target-valid.json",
        ),
        (
            "fixtures/target-repo/red/missing-product-cohesion",
            "target-red.json",
        ),
    ] {
        let target_args = vec![
            "--root".into(),
            ".".into(),
            "audit".into(),
            "--target-repo".into(),
            fixture.into(),
            "--receipt".into(),
            temp.join(receipt_name).display().to_string(),
            "--require-observability".into(),
            "--require-product-cohesion".into(),
        ];
        assert_ne!(run(&root, &target_args).status.code(), None);
    }

    std::fs::remove_dir_all(&temp).expect("remove temp root");
}

fn assert_fail_closed_cli_receipt(path: &Path, operation: &str) {
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).expect("read cli receipt"))
            .expect("parse cli receipt");
    assert_eq!(
        value["schema"],
        "harness-ultragoal.cli-control-plane-receipt.v1"
    );
    assert_eq!(value["status"], "fail");
    assert_eq!(value["operation"], operation);
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");
    assert_eq!(value["failure"]["law_id"], "cli-self-law-compliance");
}
