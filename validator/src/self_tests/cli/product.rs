use std::path::PathBuf;

#[test]
fn product_receipt_minter_rebinds_canonical_source_local_receipts() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let rel = PathBuf::from(format!(
        "target/ultragoal-product-receipts-{}",
        std::process::id()
    ));
    let out = root.join(&rel);
    let _ = std::fs::remove_dir_all(&out);

    let report = crate::cli::product::receipts::mint_all(&root, &rel, &out).expect("mint");
    assert_eq!(report["status"], "pass");
    assert!(report["failures"].as_array().expect("failures").is_empty());

    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let fit = crate::json_boundary::read_json(&out.join("fit-repo-receipt.json")).expect("fit");
    let product =
        crate::json_boundary::read_json(&out.join("product-fitness-receipt.json")).expect("pf");
    let journey = crate::json_boundary::read_json(&out.join("plugin-product-journey-receipt.json"))
        .expect("journey");

    for value in [&fit, &product, &journey] {
        assert_eq!(value["target_revision"]["value"], current);
    }
    assert_eq!(
        fit["receipt_digest"],
        crate::audit::fit_repo_receipt::canonical_digest(&fit)
    );
    assert_eq!(
        product["receipt_digest"],
        crate::audit::product::fitness::receipt::canonical_digest(&product)
    );
    assert_eq!(
        journey["evidence"][0]["path"],
        format!("{}/fit-repo-receipt.json", rel.display())
    );

    std::fs::remove_dir_all(out).expect("cleanup");
}

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
fn product_command_rejects_unsafe_receipt_dirs() {
    let root = crate::self_tests::boundaries::support::repo_root();
    for receipt_dir in [
        root.join("target/product-absolute"),
        PathBuf::from("validation_artifacts/../harness"),
    ] {
        let err =
            crate::cli::product::run(&root, &crate::cli::product::ProductCommand { receipt_dir })
                .expect_err("unsafe dir");
        assert!(
            err.contains("root-relative") || err.contains("escapes package root"),
            "{err}"
        );
    }
}

#[test]
fn product_command_runs_typed_receipt_minter_and_returns_pass_exit() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let rel = PathBuf::from(format!(
        "target/ultragoal-product-command-{}",
        std::process::id()
    ));
    let out = root.join(&rel);
    let _ = std::fs::remove_dir_all(&out);
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            receipt_dir: rel.clone(),
        },
    )
    .expect("product run succeeds");
    assert_eq!(code, 0);
    assert!(out.join("fit-repo-receipt.json").is_file());
    assert!(out.join("product-fitness-receipt.json").is_file());
    assert!(out.join("plugin-product-journey-receipt.json").is_file());
    std::fs::remove_dir_all(out).expect("cleanup product command");
}

#[test]
fn product_parse_returns_none_for_other_command_families() {
    let args = ["registry", "probe"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert!(crate::cli::product::parse(&args).expect("parse").is_none());
}
