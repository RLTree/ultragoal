use std::path::PathBuf;

#[test]
fn line_caps_parser_rejects_invalid_jobs() {
    let err = crate::parse_command(
        &["line-caps", "check", "--strict", "--jobs", "abc"]
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
fn line_caps_scheduler_rejects_unbounded_zero_jobs() {
    let root = super::package_root(
        "line-caps-zero-jobs",
        &[("validator/src/lib.rs", "pub fn ok() {}\n".to_string())],
    );
    let err = crate::cli::line_caps::run(
        &root,
        &crate::cli::line_caps::LineCapsCommand {
            receipt: PathBuf::from("validation_artifacts/observability/line-cap-check.json"),
            jobs: Some(0),
        },
    )
    .expect_err("zero jobs rejected");
    assert!(err.contains("scheduler jobs must be at least 1"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup zero jobs");
}

#[test]
fn line_caps_reports_receipt_write_failures() {
    let root = super::package_root(
        "line-caps-receipt-write-fail",
        &[("validator/src/lib.rs", "pub fn ok() {}\n".to_string())],
    );
    let blocker = root.join("target/line-cap-blocker");
    std::fs::create_dir_all(root.join("target")).expect("target dir");
    std::fs::write(&blocker, "not a directory").expect("blocker file");
    let err = crate::cli::line_caps::run(
        &root,
        &crate::cli::line_caps::LineCapsCommand {
            receipt: PathBuf::from("target/line-cap-blocker/receipt.json"),
            jobs: Some(1),
        },
    )
    .expect_err("receipt write failure");
    assert!(err.contains("create parent failed"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup write fail");
}
