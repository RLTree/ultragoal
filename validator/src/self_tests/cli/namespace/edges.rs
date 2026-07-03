use serde_json::json;
use std::path::PathBuf;

#[test]
fn namespace_command_reports_missing_manifest_and_registry_edges() {
    let missing_manifest =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-no-manifest");
    std::fs::create_dir_all(&missing_manifest).expect("temp root");
    let err = crate::cli::namespace::run(
        &missing_manifest,
        &crate::cli::namespace::NamespaceCommand {
            receipt: PathBuf::from("validation_artifacts/observability/namespace-check.json"),
            jobs: Some(1),
        },
    )
    .expect_err("missing manifest blocks candidate receipt");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    std::fs::remove_dir_all(missing_manifest).expect("cleanup missing manifest");

    let missing_registry =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-no-registry");
    super::write_file(
        &missing_registry.join("validator/src/domain/leaf.rs"),
        "pub fn leaf() {}\n",
    );
    super::write_json(
        &missing_registry.join("plugin-manifest-draft.json"),
        &json!({"resources":["validator/src/domain/leaf.rs"]}),
    );
    let code = crate::command_run::run_with_exit_code(super::args(
        missing_registry.clone(),
        &["namespace", "check", "--strict"],
    ))
    .expect("missing registry receipt");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(
        &missing_registry.join("validation_artifacts/observability/namespace-check.json"),
    )
    .expect("receipt");
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("namespace_class_registry_file_missing")
    );
    std::fs::remove_dir_all(missing_registry).expect("cleanup missing registry");
}

#[test]
fn namespace_parser_rejects_invalid_jobs() {
    let err = crate::parse_command(
        &["namespace", "check", "--strict", "--jobs", "abc"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("invalid jobs rejected");
    assert!(
        err.contains("invalid numeric value for --jobs: abc"),
        "{err}"
    );
}

#[test]
fn namespace_scheduler_rejects_unbounded_zero_jobs() {
    let root = super::package_root("namespace-zero-jobs", &[]);
    let err = crate::cli::namespace::run(
        &root,
        &crate::cli::namespace::NamespaceCommand {
            receipt: PathBuf::from("validation_artifacts/observability/namespace-check.json"),
            jobs: Some(0),
        },
    )
    .expect_err("zero jobs rejected");
    assert!(err.contains("scheduler jobs must be at least 1"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup zero jobs");
}

#[test]
fn namespace_reports_receipt_write_failures() {
    let root = super::package_root("namespace-receipt-write-fail", &[]);
    let blocker = root.join("target/namespace-blocker");
    std::fs::create_dir_all(root.join("target")).expect("target dir");
    std::fs::write(&blocker, "not a directory").expect("blocker file");
    let err = crate::cli::namespace::run(
        &root,
        &crate::cli::namespace::NamespaceCommand {
            receipt: PathBuf::from("target/namespace-blocker/receipt.json"),
            jobs: Some(1),
        },
    )
    .expect_err("receipt write failure");
    assert!(err.contains("create parent failed"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup write fail");
}
