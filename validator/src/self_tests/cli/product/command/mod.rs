use std::path::PathBuf;

mod edges;
mod fit_repo;
mod journey;

#[test]
fn product_command_rejects_unsafe_receipt_dirs() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    for receipt_dir in [
        root.join("target/product-absolute"),
        PathBuf::from("validation_artifacts/../harness"),
    ] {
        let obs = PathBuf::from(format!(
            "validation_artifacts/observability/product-observability-fail-{}-{}.json",
            std::process::id(),
            receipt_dir.to_string_lossy().len()
        ));
        let code = crate::cli::product::run(
            &root,
            &crate::cli::product::ProductCommand {
                operation: crate::cli::product::ProductOperation::ProductProveFitness,
                receipt_dir: Some(receipt_dir),
                observability_receipt: obs.clone(),
            },
        )
        .expect("unsafe dir emits fail-closed observability");
        assert_eq!(code, 1);
        let receipt = crate::json_boundary::read_json(&root.join(&obs)).expect("receipt");
        assert_eq!(receipt["status"], "fail");
        assert_eq!(receipt["event"]["failure_class"], "product_command_failure");
        assert!(
            receipt["why_failed"]
                .as_str()
                .expect("why")
                .contains("product receipt directory")
        );
    }
}

#[test]
fn product_command_runs_typed_receipt_minter_and_returns_pass_exit() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let rel = PathBuf::from(format!(
        "validation_artifacts/product/ultragoal-product-command-{}",
        std::process::id()
    ));
    let obs = rel.join("product-observability.json");
    let out = root.join(&rel);
    let _ = std::fs::remove_dir_all(&out);
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveFitness,
            receipt_dir: Some(rel.clone()),
            observability_receipt: obs.clone(),
        },
    )
    .expect("product run succeeds");
    assert_eq!(code, 0);
    assert!(out.join("fit-repo-receipt.json").is_file());
    assert!(out.join("product-fitness-receipt.json").is_file());
    assert!(out.join("plugin-product-journey-receipt.json").is_file());
    let receipt = crate::json_boundary::read_json(&root.join(&obs)).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["event"]["operation"], "product.prove-fitness");
    assert_eq!(
        receipt["check_id"],
        "product-prove-fitness-observability-binding"
    );
    assert_eq!(receipt["claim_id"], "product_fitness");
    assert_eq!(receipt["event"]["worker_count"], 1);
    assert_eq!(receipt["event"]["task_count"], 3);
    assert_eq!(
        receipt["event"]["saturation_status"],
        "shared_authority_write_serial_product_receipts"
    );
    assert!(
        receipt["trace"]["child_spans"]
            .as_array()
            .expect("child spans")
            .iter()
            .all(|span| span["parent_span_id"] == receipt["trace"]["span_id"])
    );
    std::fs::remove_dir_all(out).expect("cleanup product command");
}

#[test]
fn product_command_emits_fail_closed_observability_for_missing_receipt_dir() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let obs = PathBuf::from(format!(
        "validation_artifacts/observability/product-missing-receipt-dir-{}.json",
        std::process::id()
    ));
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveFitness,
            receipt_dir: None,
            observability_receipt: obs.clone(),
        },
    )
    .expect("missing receipt dir emits telemetry");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(&obs)).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["event"]["failure_class"], "product_command_failure");
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("missing required argument --receipt-dir")
    );
    std::fs::remove_file(root.join(obs)).expect("cleanup missing receipt dir");
}

#[test]
fn product_command_reports_minter_write_failures_with_observability() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let rel = PathBuf::from(format!(
        "validation_artifacts/product/product-output-file-{}",
        std::process::id()
    ));
    let output_file = root.join(&rel);
    std::fs::write(&output_file, "not a directory").expect("output blocker");
    let obs = PathBuf::from(format!(
        "validation_artifacts/observability/product-minter-failure-{}.json",
        std::process::id()
    ));
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveFitness,
            receipt_dir: Some(rel.clone()),
            observability_receipt: obs.clone(),
        },
    )
    .expect("minter write failure emits telemetry");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(&obs)).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["event"]["failure_class"], "product_command_failure");
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("create parent failed")
    );
    std::fs::remove_file(output_file).expect("cleanup output blocker");
    std::fs::remove_file(root.join(obs)).expect("cleanup minter failure");
}

#[test]
fn product_command_reports_candidate_digest_errors_before_receipt_claim() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("product-no-manifest");
    std::fs::create_dir_all(&root).expect("temp root");
    let err = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveCohesion,
            receipt_dir: None,
            observability_receipt: PathBuf::from(
                "validation_artifacts/observability/product-prove-cohesion.json",
            ),
        },
    )
    .expect_err("missing manifest blocks product telemetry");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup no manifest");
}

#[test]
fn product_cohesion_command_emits_fail_closed_observability_for_missing_artifacts() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("product-cohesion-command");
    std::fs::create_dir_all(&root).expect("temp root");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        r#"{"resources":[]}"#,
    )
    .expect("manifest");
    let obs = PathBuf::from("validation_artifacts/observability/product-prove-cohesion.json");
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveCohesion,
            receipt_dir: None,
            observability_receipt: obs.clone(),
        },
    )
    .expect("cohesion command emits telemetry");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(obs)).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["event"]["operation"], "product.prove-cohesion");
    assert_eq!(
        receipt["event"]["failure_class"],
        "product_cohesion_failure"
    );
    assert_eq!(
        receipt["check_id"],
        "product-prove-cohesion-observability-binding"
    );
    assert_eq!(receipt["claim_id"], "product_cohesion");
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("requested product cohesion gate missing")
    );
    assert!(
        receipt["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("product prove-cohesion")
    );
    std::fs::remove_dir_all(root).expect("cleanup cohesion command");
}
