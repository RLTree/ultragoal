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

fn ultragoal() -> &'static str {
    env!("CARGO_BIN_EXE_ultragoal")
}

fn run(root: &Path, args: &[String]) -> std::process::Output {
    run_bin(ultragoal(), root, args)
}

fn run_bin(bin: &str, root: &Path, args: &[String]) -> std::process::Output {
    Command::new(bin)
        .current_dir(root)
        .args(args)
        .output()
        .expect("validator command runs")
}

fn root_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("test output path is inside command root")
        .to_string_lossy()
        .into_owned()
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
        run(
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
        assert_eq!(run(&root, &args).status.code(), Some(2));
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
        run(&root, &review_round_parse_success).status.code(),
        Some(1)
    );

    let digest = run(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "package".into(),
            "digest".into(),
        ],
    );
    assert!(digest.status.success(), "package digest failed: {digest:?}");
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

    let pid = std::process::id();
    let product_receipt_dir = PathBuf::from(format!(
        "validation_artifacts/product/cli-surface-receipts-{pid}"
    ));
    let product = run(
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
        run(
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
        run(
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
    let target_parse = run(
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
    std::fs::create_dir_all(temp.join("fixtures/valid")).expect("valid fixtures");
    std::fs::create_dir_all(temp.join("fixtures/red")).expect("red fixtures");
    std::fs::create_dir_all(temp.join("templates")).expect("templates");
    std::fs::write(
        temp.join("fixtures/valid/minimal-goal-run.json"),
        r#"{"claims":[]}"#,
    )
    .expect("base fixture");
    std::fs::write(
        temp.join("fixtures/red/missing-patch.json"),
        r#"{
  "expected_failure": {
    "check_id": "red-fixture-coverage",
    "error": "red_fixture_json_patch_missing"
  },
  "base_fixture_path": "fixtures/valid/minimal-goal-run.json"
}"#,
    )
    .expect("red packet");
    std::fs::write(
        temp.join("templates/RED_FIXTURES.json"),
        r#"[{
  "id": "missing-patch",
  "packet_path": "fixtures/red/missing-patch.json",
  "expected_failure": {
    "check_id": "red-fixture-coverage",
    "error": "red_fixture_json_patch_missing"
  }
}]"#,
    )
    .expect("red catalog");
    let red_report = temp.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    std::fs::create_dir_all(red_report.parent().expect("red report parent"))
        .expect("red report dir");
    let red_report_run = run(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "red-fixture-report".into(),
            "--report".into(),
            root_relative(&temp, &red_report),
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

    let performance_receipt = root.join(format!(
        "validation_artifacts/cli/cli-surface-performance-{pid}.json"
    ));
    let performance = run(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "self".into(),
            "performance".into(),
            "prove".into(),
            "--receipt".into(),
            root_relative(&root, &performance_receipt),
        ],
    );
    assert_eq!(
        performance.status.code(),
        Some(1),
        "performance command should fail closed without node speed evidence: {performance:?}"
    );
    let performance_value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&performance_receipt).expect("performance receipt"))
            .expect("performance receipt json");
    assert_eq!(
        performance_value["failure"]["id"],
        "cli_performance_missing_node_speed_proof"
    );

    let rust_fast_receipt = root.join(format!(
        "validation_artifacts/rust/cli-surface-fast-{pid}.json"
    ));
    let rust_fast = run(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "rust".into(),
            "fast".into(),
            "--receipt".into(),
            root_relative(&root, &rust_fast_receipt),
        ],
    );
    assert!(
        rust_fast.status.success(),
        "rust fast failed: {rust_fast:?}"
    );

    let gc_plan_receipt = root.join(format!(
        "validation_artifacts/gc/cli-surface-plan-{pid}.json"
    ));
    let gc_plan = run(
        &root,
        &[
            "--root".into(),
            ".".into(),
            "gc".into(),
            "plan".into(),
            "--receipt".into(),
            root_relative(&root, &gc_plan_receipt),
        ],
    );
    assert!(gc_plan.status.success(), "gc plan failed: {gc_plan:?}");

    let update_goal_receipt = temp.join("validation_artifacts/cli/update-goal-eligibility.json");
    let update_goal = run(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "update-goal".into(),
            "eligibility".into(),
            "--receipt".into(),
            "validation_artifacts/cli/update-goal-eligibility.json".into(),
        ],
    );
    assert_eq!(update_goal.status.code(), Some(1));
    assert_fail_closed_cli_receipt(&update_goal_receipt, "update_goal_eligibility");

    let self_receipt = temp.join("validation_artifacts/cli/self-law-receipt.json");
    let self_law = run(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "self".into(),
            "update-goal".into(),
            "eligibility".into(),
            "--receipt".into(),
            "validation_artifacts/cli/self-law-receipt.json".into(),
        ],
    );
    assert_eq!(self_law.status.code(), Some(1));
    assert_fail_closed_cli_receipt(&self_receipt, "self_update_goal_eligibility");

    assert_eq!(
        run(
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
    let transaction = run(
        &root,
        &[
            "--root".into(),
            temp.display().to_string(),
            "transaction".into(),
            "finalize".into(),
            "--receipt".into(),
            "validation_artifacts/cli/transactional-finalization.json".into(),
        ],
    );
    assert_eq!(transaction.status.code(), Some(1));
    assert_fail_closed_transaction_receipt(&transaction_receipt);

    let review_target = root.join(format!(
        "validation_artifacts/review/cli-surface-review-target-{pid}.json"
    ));
    let review_target_args = vec![
        "--root".into(),
        ".".into(),
        "review-target".into(),
        "--receipt".into(),
        root_relative(&root, &review_target),
    ];
    assert!(run(&root, &review_target_args).status.success());

    let archive_args = vec![
        "--root".into(),
        ".".into(),
        "archive".into(),
        "--zip".into(),
        format!("validation_artifacts/review/cli-surface-candidate-{pid}.zip"),
        "--receipt".into(),
        format!("validation_artifacts/review/cli-surface-archive-{pid}.json"),
    ];
    assert!(run(&root, &archive_args).status.success());

    let semantic_args = vec![
        "--root".into(),
        ".".into(),
        "semantic-receipts".into(),
        "--input".into(),
        "fixtures/valid/minimal-goal-run.json".into(),
        "--out-dir".into(),
        format!("validation_artifacts/semantic/cli-surface-normal-{pid}"),
    ];
    assert!(run(&root, &semantic_args).status.success());

    let refused_model = vec![
        "--root".into(),
        ".".into(),
        "semantic-receipts".into(),
        "--input".into(),
        "fixtures/valid/minimal-goal-run.json".into(),
        "--out-dir".into(),
        format!("validation_artifacts/semantic/cli-surface-model-refusal-{pid}"),
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
        format!("validation_artifacts/semantic/cli-surface-red-{pid}"),
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
            format!("validation_artifacts/target-repo/cli-surface-{pid}-{receipt_name}"),
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
