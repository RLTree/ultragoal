use serde_json::json;

fn write_json(path: &std::path::Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn minimal_root(name: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(name);
    for dir in [
        "examples/generated",
        "fixtures/valid",
        "schemas",
        "templates",
        "validation_artifacts/ultragoal-audit",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[]}),
    );
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));
    write_json(
        &root.join("fixtures/valid/minimal-goal-run.json"),
        &json!({}),
    );
    root
}

fn audit_options(
    root: std::path::PathBuf,
    receipt: std::path::PathBuf,
    red_report: std::path::PathBuf,
) -> crate::audit::AuditOptions {
    crate::audit::AuditOptions {
        root,
        receipt,
        red_report: Some(red_report),
        target_repo: None,
        mode: "source".to_string(),
        require_observability: false,
        require_product_cohesion: false,
        command_text: "ultragoal source audit --unit".to_string(),
    }
}

#[test]
#[cfg(unix)]
fn package_run_propagates_artifact_and_output_authority_errors() {
    let artifact_root = minimal_root("package-run-hardlink-artifact");
    std::fs::write(artifact_root.join("Cargo.toml"), "[package]\nname='x'\n").expect("cargo toml");
    std::fs::hard_link(
        artifact_root.join("Cargo.toml"),
        artifact_root.join("Cargo-hardlink.toml"),
    )
    .expect("hard link");
    let artifact_err = crate::audit::package::run::run(
        audit_options(
            artifact_root.clone(),
            artifact_root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
            artifact_root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json"),
        ),
        artifact_root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json"),
    )
    .expect_err("hard-linked validator artifact rejected");
    assert!(
        artifact_err.contains("hard-linked file rejected"),
        "{artifact_err}"
    );
    std::fs::remove_dir_all(artifact_root).expect("cleanup artifact root");

    let red_root = minimal_root("package-run-red-report-dir");
    let red_report = red_root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    std::fs::create_dir(&red_report).expect("red report dir");
    let red_err = crate::audit::package::run::run(
        audit_options(
            red_root.clone(),
            red_root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
            red_report.clone(),
        ),
        red_report,
    )
    .expect_err("red report directory rejected");
    assert!(red_err.contains("json rename failed"), "{red_err}");
    std::fs::remove_dir_all(red_root).expect("cleanup red root");

    let receipt_root = minimal_root("package-run-receipt-symlink");
    let real_parent = receipt_root.join("real-receipts");
    std::fs::create_dir_all(&real_parent).expect("real parent");
    let link_parent = receipt_root.join("receipt-link");
    std::os::unix::fs::symlink(&real_parent, &link_parent).expect("receipt symlink");
    let receipt_err = crate::audit::package::run::run(
        audit_options(
            receipt_root.clone(),
            link_parent.join("validator-receipt.json"),
            receipt_root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json"),
        ),
        receipt_root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json"),
    )
    .expect_err("stdio receipt symlink rejected");
    assert!(
        receipt_err.contains("output path uses symlink"),
        "{receipt_err}"
    );
    std::fs::remove_dir_all(receipt_root).expect("cleanup receipt root");
}
