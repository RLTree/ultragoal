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

fn package_digest_from_stdout(output: &std::process::Output) -> String {
    assert!(
        output.status.success(),
        "package digest command failed: {output:?}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find(|line| line.starts_with("sha256:") && line.len() == "sha256:".len() + 64)
        .expect("raw package digest line")
        .to_string()
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
    assert_eq!(
        run_ultragoal(
            &root,
            &["--root".into(), ".".into(), "source".into(), "audit".into(),],
        )
        .status
        .code(),
        Some(2)
    );
    for args in [
        vec![
            "--root".into(),
            ".".into(),
            "archive".into(),
            "build".into(),
            "--receipt".into(),
            temp.join("archive-missing-zip.json").display().to_string(),
        ],
        vec![
            "--root".into(),
            ".".into(),
            "archive".into(),
            "build".into(),
            "--zip".into(),
            temp.join("archive-missing-receipt.zip")
                .display()
                .to_string(),
        ],
        vec![
            "--root".into(),
            ".".into(),
            "review-round".into(),
            "verify".into(),
            "--receipt".into(),
            temp.join("review-round-missing-anchor.json")
                .display()
                .to_string(),
        ],
        vec![
            "--root".into(),
            ".".into(),
            "semantic-receipts".into(),
            "--out-dir".into(),
            temp.join("semantic-missing-input").display().to_string(),
        ],
        vec![
            "--root".into(),
            ".".into(),
            "semantic-receipts".into(),
            "--input".into(),
            "fixtures/valid/minimal-goal-run.json".into(),
        ],
    ] {
        assert_eq!(run_ultragoal(&root, &args).status.code(), Some(2));
    }

    let review_round_parse_success = vec![
        "--root".into(),
        ".".into(),
        "review-round".into(),
        "verify".into(),
        "--receipt".into(),
        temp.join("review-round-missing-input.json")
            .display()
            .to_string(),
        "--validator-receipt".into(),
        "fixtures/review-round/anchors/validator-receipt.json".into(),
        "--review-target-receipt".into(),
        "fixtures/review-round/anchors/review-target-receipt.json".into(),
        "--archive-receipt".into(),
        "fixtures/review-round/anchors/archive-receipt.json".into(),
    ];
    assert_eq!(
        run_ultragoal(&root, &review_round_parse_success)
            .status
            .code(),
        Some(1)
    );

    let digest = run(
        &root,
        &["--root".into(), ".".into(), "package-digest".into()],
    );
    assert!(digest.status.success(), "package-digest failed: {digest:?}");
    let digest_stdout = String::from_utf8_lossy(&digest.stdout);
    assert!(
        digest_stdout.lines().next().is_some_and(|line| {
            line.starts_with("sha256:") && line.len() == "sha256:".len() + 64
        }),
        "package digest first line is not raw digest: {digest_stdout}"
    );
    assert!(
        digest_stdout.contains("receipt=validation_artifacts/observability/package-digest.json")
    );
    assert!(digest_stdout.contains("run_id=run-"));

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

    let product_receipt_dir = temp
        .strip_prefix(&root)
        .expect("temp under root")
        .join("product-receipts");
    let product = run_ultragoal(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "product".into(),
            "prove-fitness".into(),
            "--receipt-dir".into(),
            product_receipt_dir.display().to_string(),
        ],
    );
    assert!(
        product.status.success(),
        "product prove-fitness failed: {product:?}"
    );
    assert!(
        root.join(&product_receipt_dir)
            .join("product-fitness-receipt.json")
            .is_file()
    );
    assert_eq!(
        run_ultragoal(
            &root,
            &[
                "--root".into(),
                ".".into(),
                "product".into(),
                "prove-fitness".into(),
                "--receipt".into(),
                temp.join("product-control.json").display().to_string(),
            ],
        )
        .status
        .code(),
        Some(2)
    );
    assert_eq!(
        run_ultragoal(
            &root,
            &[
                "--root".into(),
                ".".into(),
                "product".into(),
                "unknown".into(),
            ],
        )
        .status
        .code(),
        Some(2)
    );

    let target_parse_root = temp.join("target-parse-observability");
    std::fs::create_dir_all(&target_parse_root).expect("target parse root");
    std::fs::write(
        target_parse_root.join("plugin-manifest-draft.json"),
        r#"{"resources":[]}"#,
    )
    .expect("target parse manifest");
    let target_parse = run_ultragoal(
        &root,
        &[
            "--root".into(),
            target_parse_root.display().to_string(),
            "target-repo".into(),
            "audit".into(),
            "--receipt".into(),
            "validation_artifacts/ultragoal-audit/target-receipt.json".into(),
        ],
    );
    assert_eq!(target_parse.status.code(), Some(2));
    let target_parse_obs =
        target_parse_root.join("validation_artifacts/observability/target-repo-audit.json");
    let target_parse_value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&target_parse_obs).expect("target parse obs"))
            .expect("target parse obs json");
    assert_eq!(target_parse_value["operation"], "target-repo.audit");
    assert_eq!(
        target_parse_value["event"]["where_failed"],
        "target-repo.audit.parse"
    );

    std::fs::write(
        temp.join("plugin-manifest-draft.json"),
        r#"{"version":"0.0.0-test","resources":[]}"#,
    )
    .expect("write temp manifest");

    let temp_digest = package_digest_from_stdout(&run_ultragoal(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "package".into(),
            "digest".into(),
        ],
    ));
    let red_report = temp.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    std::fs::create_dir_all(red_report.parent().expect("red report parent"))
        .expect("red report dir");
    std::fs::write(
        &red_report,
        format!(
            r#"{{
  "schema": "harness-ultragoal.red-fixture-report.v1",
  "status": "pass",
  "target_revision": {{"kind": "package_digest", "value": "{temp_digest}"}},
  "red_fixtures": {{"red-one": {{"status": "pass"}}}}
}}"#
        ),
    )
    .expect("red report");
    let red_report_run = run_ultragoal(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "red-fixture-report".into(),
            "--report".into(),
            red_report.display().to_string(),
        ],
    );
    assert!(
        red_report_run.status.success(),
        "red fixture report failed: {red_report_run:?}"
    );
    let red_observe = temp.join("validation_artifacts/observability/red-fixture-report.json");
    let red_value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&red_observe).expect("red observe"))
            .expect("red observe json");
    assert_eq!(red_value["operation"], "red_fixture.report");
    assert_eq!(red_value["event"]["command"], "ultragoal red");

    let performance_receipt = temp.join("validation_artifacts/cli/performance-receipt.json");
    let performance = run_ultragoal(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "performance".into(),
            "prove".into(),
            "--receipt".into(),
            performance_receipt.display().to_string(),
        ],
    );
    assert!(
        performance.status.success(),
        "performance command failed: {performance:?}"
    );

    let rust_fast_receipt = temp.join("validation_artifacts/rust/fast-receipt.json");
    let rust_fast = run_ultragoal(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "rust".into(),
            "fast".into(),
            "--receipt".into(),
            rust_fast_receipt.display().to_string(),
        ],
    );
    assert!(
        rust_fast.status.success(),
        "rust fast failed: {rust_fast:?}"
    );

    let gc_plan_receipt = temp.join("validation_artifacts/gc/plan-receipt.json");
    let gc_plan = run_ultragoal(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "gc".into(),
            "plan".into(),
            "--receipt".into(),
            gc_plan_receipt.display().to_string(),
        ],
    );
    assert!(gc_plan.status.success(), "gc plan failed: {gc_plan:?}");

    let update_goal_receipt = temp.join("validation_artifacts/cli/update-goal-eligibility.json");
    let update_goal = run_ultragoal(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "update-goal".into(),
            "eligibility".into(),
            "--receipt".into(),
            update_goal_receipt.display().to_string(),
        ],
    );
    assert_eq!(update_goal.status.code(), Some(1));
    assert_fail_closed_cli_receipt(&update_goal_receipt, "update_goal_eligibility");

    let self_receipt = temp.join("validation_artifacts/cli/self-law-receipt.json");
    let self_law = run_ultragoal(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "self".into(),
            "update-goal".into(),
            "eligibility".into(),
            "--receipt".into(),
            self_receipt.display().to_string(),
        ],
    );
    assert_eq!(self_law.status.code(), Some(1));
    assert_fail_closed_cli_receipt(&self_receipt, "self_update_goal_eligibility");

    assert_eq!(
        run_ultragoal(
            &root,
            &[
                "--root".into(),
                temp.display().to_string(),
                "transaction".into(),
                "finalize".into(),
            ],
        )
        .status
        .code(),
        Some(2)
    );

    let transaction_receipt = temp.join("validation_artifacts/cli/transactional-finalization.json");
    let transaction = run_ultragoal(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "transaction".into(),
            "finalize".into(),
            "--receipt".into(),
            transaction_receipt.display().to_string(),
        ],
    );
    assert_eq!(transaction.status.code(), Some(1));
    assert_fail_closed_transaction_receipt(&transaction_receipt);

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
    let expected_law = if operation == "self_update_goal_eligibility" {
        "cli-self-law-compliance"
    } else {
        "cli-control-plane-authority"
    };
    assert_eq!(value["failure"]["law_id"], expected_law);
}

fn assert_fail_closed_transaction_receipt(path: &Path) {
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).expect("read transaction receipt"))
            .expect("parse transaction receipt");
    assert_eq!(
        value["schema"],
        "harness-ultragoal.cli-transactional-finalization-receipt.v1"
    );
    assert_eq!(value["status"], "fail");
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");
    assert!(
        value["failure"]["observed_failures"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}
