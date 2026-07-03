use serde_json::Value;

fn args(root: std::path::PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or("")
}

#[test]
fn review_round_command_emits_fail_closed_observability_for_stale_anchors() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let observability_receipt = format!(
        "target/review-round-stale-anchor-observability-{}.json",
        std::process::id()
    );
    let receipt = root
        .join("fixtures/review-round/valid/review-round-receipt.json")
        .to_string_lossy()
        .to_string();
    let validator_receipt = root
        .join("fixtures/review-round/anchors/validator-receipt.json")
        .to_string_lossy()
        .to_string();
    let review_target_receipt = root
        .join("fixtures/review-round/anchors/review-target-receipt.json")
        .to_string_lossy()
        .to_string();
    let archive_receipt = root
        .join("fixtures/review-round/anchors/archive-receipt.json")
        .to_string_lossy()
        .to_string();
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "review-round",
            "verify",
            "--receipt",
            &receipt,
            "--validator-receipt",
            &validator_receipt,
            "--review-target-receipt",
            &review_target_receipt,
            "--archive-receipt",
            &archive_receipt,
            "--observability-receipt",
            &observability_receipt,
        ],
    ))
    .expect("review round command emits fail-closed telemetry");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(&observability_receipt))
        .expect("observability receipt");
    assert_eq!(text(&receipt, "status"), "fail");
    assert_eq!(text(&receipt, "operation"), "review-round.verify");
    assert_eq!(
        text(&receipt, "check_id"),
        "review-round-verify-observability-binding"
    );
    assert_eq!(
        text(&receipt, "failure_class"),
        "review_round_validation_failure"
    );
    assert!(text(&receipt, "why_failed").contains("review_round_anchor_source_mismatch"));
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim == "update_goal_eligibility")
    );
    assert_eq!(receipt["event"]["worker_count"], 1);
    assert_eq!(receipt["event"]["task_count"], 4);
    assert_eq!(receipt["event"]["queue_depth"], 4);
    assert_eq!(
        text(&receipt["event"], "saturation_status"),
        "shared_authority_read_serial_review_round_anchors"
    );
    std::fs::remove_file(root.join(observability_receipt)).expect("cleanup observability receipt");
}

#[test]
fn review_round_observability_receipt_path_validation_is_fail_closed() {
    for raw in [
        vec!["--observability-receipt", ""],
        vec!["--observability-receipt", "../outside.json"],
    ] {
        let args = raw.iter().map(|item| item.to_string()).collect::<Vec<_>>();
        let error = crate::cli::review::round::observability_receipt(&args)
            .expect_err("unsafe observability receipt rejected");
        assert!(
            error.contains("review round observability receipt"),
            "{error}"
        );
    }
}
