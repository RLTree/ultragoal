#[test]
fn target_receipt_generation_records_all_fixture_outputs() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let rel = std::path::PathBuf::from(format!(
        "validation_artifacts/target-repo/test-receipts-{}",
        std::process::id()
    ));
    let receipt_dir = root.join(&rel);
    let _ = std::fs::remove_dir_all(&receipt_dir);
    let generated =
        crate::target_fixtures::write_target_receipts(&root, &rel, &[]).expect("target receipts");
    assert!(!generated.is_empty());
    assert!(generated.iter().all(|item| {
        item["artifact_type"] == "target_repo_receipt"
            && item["digest"].as_str().unwrap_or("").starts_with("sha256:")
            && item["path"].as_str().is_some_and(|path| {
                path.starts_with("validation_artifacts/target-repo/test-receipts-")
            })
    }));
    let blocked_receipt_dir = receipt_dir.join("blocked-file");
    std::fs::write(&blocked_receipt_dir, "not a directory").expect("blocked receipt directory");
    let err = crate::target_fixtures::write_target_receipts(&root, &blocked_receipt_dir, &[])
        .expect_err("file output dir blocks receipt writes");
    assert!(err.contains("create parent failed"), "{err}");
    let outside_receipt_dir = std::env::temp_dir().join(format!(
        "ultragoal-target-receipts-outside-{}",
        std::process::id()
    ));
    let err = crate::target_fixtures::write_target_receipts(&root, &outside_receipt_dir, &[])
        .expect_err("outside-root absolute receipt directory is not claim authority");
    assert!(err.contains("must stay inside package root"), "{err}");
    std::fs::remove_dir_all(receipt_dir).expect("cleanup generated receipts");
}

#[test]
fn symlink_fixture_os_error_mappers_are_testable() {
    use std::io::Error;

    assert!(
        crate::target_fixtures::read_link_result(
            Err(Error::other("denied")),
            "symlink fixture readlink failed",
        )
        .expect_err("readlink failure")
        .contains("readlink failed")
    );
    assert!(
        crate::target_fixtures::remove_result(
            Err(Error::other("busy")),
            "symlink fixture cleanup failed",
        )
        .expect_err("remove failure")
        .contains("cleanup failed")
    );
    assert!(
        crate::target_fixtures::symlink_result(
            Err(Error::other("blocked")),
            "symlink fixture materialize failed",
        )
        .expect_err("symlink failure")
        .contains("materialize failed")
    );
    let tmp = std::env::temp_dir().join(format!("ultragoal-symlink-parent-{}", std::process::id()));
    crate::target_fixtures::create_symlink_parent(&tmp.join("nested"))
        .expect("create symlink parent");
    std::fs::remove_dir_all(tmp).expect("cleanup symlink parent");
}

#[test]
fn target_capability_failures_report_expected_mismatch_and_missing_fixture() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let specs = [
        crate::target_fixtures::TargetSpec {
            name: "valid-init-as-red.json",
            rel: "fixtures/target-repo/valid-init",
            mode: "init",
            expected_code: 1,
            require_observability: false,
            require_product: false,
            expected_check: Some("check-gate"),
            expected_status: Some("fail"),
        },
        crate::target_fixtures::TargetSpec {
            name: "missing-target.json",
            rel: "fixtures/target-repo/not-present",
            mode: "init",
            expected_code: 0,
            require_observability: false,
            require_product: false,
            expected_check: None,
            expected_status: None,
        },
    ];
    let failures =
        crate::target_fixtures::target_capability_failures_for(root.as_path(), &[], &specs);
    assert!(failures.iter().any(|item| item.contains("expected exit 1")));
    assert!(
        failures
            .iter()
            .any(|item| item.contains("expected check-gate=fail"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("missing target repo fixture"))
    );
}
