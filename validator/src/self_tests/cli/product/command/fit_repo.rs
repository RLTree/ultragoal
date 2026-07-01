use std::path::PathBuf;

#[test]
fn fit_repo_command_runs_minter_with_observability() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let rel = PathBuf::from(format!(
        "target/ultragoal-fit-repo-command-{}",
        std::process::id()
    ));
    let obs = rel.join("fit-repo-observability.json");
    let out = root.join(&rel);
    let _ = std::fs::remove_dir_all(&out);
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::FitRepoProve,
            receipt_dir: Some(rel.clone()),
            observability_receipt: obs.clone(),
        },
    )
    .expect("fit-repo run succeeds");
    assert_eq!(code, 0);
    assert!(out.join("fit-repo-receipt.json").is_file());
    let receipt = crate::json_boundary::read_json(&root.join(&obs)).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["event"]["operation"], "fit-repo.prove");
    assert_eq!(receipt["check_id"], "fit-repo-prove-observability-binding");
    assert_eq!(receipt["claim_id"], "fit_repo");
    assert_eq!(receipt["supported_claims"][0], "fit_repo");
    assert_eq!(receipt["event"]["task_count"], 3);
    std::fs::remove_dir_all(out).expect("cleanup fit-repo command");
}

#[test]
fn fit_repo_command_emits_fail_closed_observability_for_invalid_receipt_dir() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let obs = PathBuf::from(format!(
        "target/fit-repo-invalid-receipt-dir-{}.json",
        std::process::id()
    ));
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::FitRepoProve,
            receipt_dir: Some(PathBuf::from("validation_artifacts/../harness")),
            observability_receipt: obs.clone(),
        },
    )
    .expect("fit-repo command emits fail-closed telemetry");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(&obs)).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["event"]["operation"], "fit-repo.prove");
    assert_eq!(receipt["event"]["failure_class"], "product_command_failure");
    assert_eq!(receipt["check_id"], "fit-repo-prove-observability-binding");
    assert_eq!(receipt["claim_id"], "fit_repo");
    assert_eq!(receipt["supported_claims"].as_array().unwrap().len(), 0);
    assert_eq!(
        receipt["claim_impact"],
        "fit_repo_failed_blocks_readiness_release_completion_update_goal"
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim == "update_goal_eligibility")
    );
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("escapes package root")
    );
    assert!(
        receipt["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("fit-repo prove")
    );
    std::fs::remove_file(root.join(obs)).expect("cleanup fit-repo failure receipt");
}
