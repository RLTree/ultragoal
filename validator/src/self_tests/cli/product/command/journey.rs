use std::path::PathBuf;

#[test]
fn product_journey_command_runs_default_minter_with_journey_telemetry() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let rel = PathBuf::from(format!(
        "validation_artifacts/product/ultragoal-product-journey-command-{}",
        std::process::id()
    ));
    let obs = rel.join("journey-observability.json");
    let out = root.join(&rel);
    let _ = std::fs::remove_dir_all(&out);
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveJourney,
            receipt_dir: Some(rel.clone()),
            observability_receipt: obs.clone(),
        },
    )
    .expect("journey run succeeds");
    assert_eq!(code, 0);
    assert!(out.join("plugin-product-journey-receipt.json").is_file());
    let receipt = crate::json_boundary::read_json(&root.join(&obs)).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["event"]["operation"], "product.prove-journey");
    assert_eq!(
        receipt["law_id"],
        "plugin-flow-graph-package-dependency-closure-plugin-product-journey"
    );
    assert_eq!(
        receipt["check_id"],
        "product-prove-journey-observability-binding"
    );
    assert_eq!(receipt["claim_id"], "plugin_product_journey");
    assert_eq!(receipt["event"]["task_count"], 3);
    std::fs::remove_dir_all(out).expect("cleanup journey command");
}

#[test]
fn product_journey_command_emits_fail_closed_observability_for_invalid_receipt_dir() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let obs = PathBuf::from(format!(
        "validation_artifacts/observability/product-journey-invalid-receipt-dir-{}.json",
        std::process::id()
    ));
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveJourney,
            receipt_dir: Some(PathBuf::from("validation_artifacts/../harness")),
            observability_receipt: obs.clone(),
        },
    )
    .expect("journey command emits fail-closed telemetry");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(&obs)).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["event"]["operation"], "product.prove-journey");
    assert_eq!(receipt["event"]["failure_class"], "product_command_failure");
    assert_eq!(
        receipt["check_id"],
        "product-prove-journey-observability-binding"
    );
    assert_eq!(receipt["claim_id"], "plugin_product_journey");
    assert_eq!(receipt["supported_claims"].as_array().unwrap().len(), 0);
    assert_eq!(
        receipt["claim_impact"],
        "plugin_product_journey_failed_blocks_readiness_release_completion_update_goal"
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
            .contains("product prove-journey")
    );
    std::fs::remove_file(root.join(obs)).expect("cleanup journey failure receipt");
}
