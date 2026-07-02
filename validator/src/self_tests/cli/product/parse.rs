use std::path::PathBuf;

#[test]
fn product_prove_fitness_requires_package_receipt_dir() {
    let args = ["product", "prove-fitness", "--receipt", "control.json"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let err = crate::cli::product::parse(&args).expect_err("missing receipt-dir");
    assert!(err.contains("--receipt-dir"));
}

#[test]
fn product_parser_keeps_product_and_fit_repo_operations_distinct() {
    let cohesion = ["product", "prove-cohesion"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let cohesion = crate::cli::product::parse(&cohesion)
        .expect("cohesion parse")
        .expect("cohesion command");
    assert_eq!(
        cohesion.operation,
        crate::cli::product::ProductOperation::ProductProveCohesion
    );

    let product = ["product", "prove-fitness", "--receipt-dir", "receipts"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let product = crate::cli::product::parse(&product)
        .expect("product parse")
        .expect("product command");
    assert_eq!(
        product.operation,
        crate::cli::product::ProductOperation::ProductProveFitness
    );

    let journey = ["product", "prove-journey"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let journey = crate::cli::product::parse(&journey)
        .expect("journey parse")
        .expect("journey command");
    assert_eq!(
        journey.operation,
        crate::cli::product::ProductOperation::ProductProveJourney
    );
    assert_eq!(
        journey.receipt_dir,
        Some(PathBuf::from("validation_artifacts/harness"))
    );

    let fit_repo = ["fit-repo", "prove", "--receipt-dir", "receipts"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let fit_repo = crate::cli::product::parse(&fit_repo)
        .expect("fit repo parse")
        .expect("fit repo command");
    assert_eq!(
        fit_repo.operation,
        crate::cli::product::ProductOperation::FitRepoProve
    );
}

#[test]
fn product_parser_rejects_missing_observability_receipt_value() {
    let args = [
        "product",
        "prove-fitness",
        "--receipt-dir",
        "receipts",
        "--observability-receipt",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let err = crate::cli::product::parse(&args).expect_err("missing receipt path fails");
    assert!(err.contains("--observability-receipt"), "{err}");

    for command in [["product", "prove-cohesion"], ["product", "prove-journey"]] {
        let args = command
            .into_iter()
            .chain(["--observability-receipt"])
            .map(str::to_string)
            .collect::<Vec<_>>();
        let err = crate::cli::product::parse(&args).expect_err("missing receipt path fails");
        assert!(err.contains("--observability-receipt"), "{err}");
    }
}

#[test]
fn product_parser_rejects_unsafe_product_cohesion_observability_receipts() {
    for receipt in [
        "/tmp/product-prove-cohesion.json",
        "validation_artifacts/../product-prove-cohesion.json",
    ] {
        let args = [
            "product",
            "prove-cohesion",
            "--observability-receipt",
            receipt,
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let err = crate::cli::product::parse(&args).expect_err("unsafe receipt path fails");
        assert!(err.contains("product observability receipt"), "{err}");
    }
}

#[test]
fn product_parser_rejects_unsafe_product_journey_observability_receipts() {
    for receipt in [
        "/tmp/product-prove-journey.json",
        "validation_artifacts/../product-prove-journey.json",
    ] {
        let args = [
            "product",
            "prove-journey",
            "--observability-receipt",
            receipt,
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let err = crate::cli::product::parse(&args).expect_err("unsafe receipt path fails");
        assert!(err.contains("product observability receipt"), "{err}");
    }
}

#[test]
fn product_parse_returns_none_for_other_command_families() {
    let args = ["registry", "probe"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert!(crate::cli::product::parse(&args).expect("parse").is_none());
}

#[test]
fn product_operation_contracts_are_distinct_for_every_surface() {
    use crate::cli::product::ProductOperation as Op;

    let rows = [
        (
            Op::ProductProveCohesion,
            "ultragoal product",
            "prove-cohesion",
            "product.prove-cohesion",
            "product_cohesion",
            "product-cohesion-gate",
            "product_cohesion_failure",
            "supports_product_cohesion_source_local_observability_only",
            "product_cohesion_failed_blocks_readiness_release_completion_update_goal",
            "validation_artifacts/observability/product-prove-cohesion.json",
        ),
        (
            Op::ProductProveFitness,
            "ultragoal product",
            "prove-fitness",
            "product.prove-fitness",
            "product_fitness",
            "product-fitness-gate",
            "product_receipt_failure",
            "supports_product_fitness_source_local_observability_only",
            "product_fitness_failed_blocks_readiness_release_completion_update_goal",
            "validation_artifacts/observability/product-prove-fitness.json",
        ),
        (
            Op::ProductProveJourney,
            "ultragoal product",
            "prove-journey",
            "product.prove-journey",
            "plugin_product_journey",
            "plugin-flow-graph-package-dependency-closure-plugin-product-journey",
            "plugin_product_journey_failure",
            "supports_plugin_product_journey_source_local_observability_only",
            "plugin_product_journey_failed_blocks_readiness_release_completion_update_goal",
            "validation_artifacts/observability/product-prove-journey.json",
        ),
        (
            Op::FitRepoProve,
            "ultragoal fit-repo",
            "prove",
            "fit-repo.prove",
            "fit_repo",
            "product-fitness-gate",
            "product_receipt_failure",
            "supports_fit_repo_source_local_observability_only",
            "fit_repo_failed_blocks_readiness_release_completion_update_goal",
            "validation_artifacts/observability/fit-repo-prove.json",
        ),
    ];

    for (
        operation,
        command,
        subcommand,
        telemetry_operation,
        claim_id,
        law_id,
        failure_class,
        pass_impact,
        fail_impact,
        receipt_rel,
    ) in rows
    {
        assert_eq!(operation.command(), command);
        assert_eq!(operation.subcommand(), subcommand);
        assert_eq!(operation.telemetry_operation(), telemetry_operation);
        assert_eq!(operation.claim_id(), claim_id);
        assert_eq!(operation.law_id(), law_id);
        assert_eq!(operation.report_failure_class(), failure_class);
        assert_eq!(operation.pass_claim_impact(), pass_impact);
        assert_eq!(operation.fail_claim_impact(), fail_impact);
        assert_eq!(operation.receipt_rel(), receipt_rel);
        assert!(operation.check_id().contains("observability-binding"));
        assert!(!operation.artifact_path().is_empty());
        assert!(operation.next_repair().contains("rerun"));
    }
}
