use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn symlink_meta(link_path: &str, target_path: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.target-fixture-symlink.v1",
        "link_path": link_path,
        "target_path": target_path
    })
}

#[test]
fn target_receipt_generation_records_all_fixture_outputs() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let rel = std::path::PathBuf::from(format!(
        "target/ultragoal-target-receipts-{}",
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
            && item["path"]
                .as_str()
                .is_some_and(|path| path.starts_with("target/ultragoal-target-receipts-"))
    }));
    let blocked_receipt_dir = receipt_dir.join("blocked-file");
    std::fs::write(&blocked_receipt_dir, "not a directory").expect("blocked receipt directory");
    let err = crate::target_fixtures::write_target_receipts(&root, &blocked_receipt_dir, &[])
        .expect_err("file output dir blocks receipt writes");
    assert!(err.contains("create parent failed"), "{err}");
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

#[test]
fn symlink_fixture_materialization_fails_closed_and_cleans_up() {
    let target =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("target-symlink-fixture");
    let meta = target.join("validation_artifacts/product-cohesion/symlink-fixture.json");
    write_json(&meta, &json!({"schema": "wrong"}));
    let err = crate::target_fixtures::materialize_symlink_fixture(&target).expect_err("bad schema");
    assert!(err.contains("schema mismatch"));

    write_json(&meta, &symlink_meta("../escape", "target.txt"));
    let err = crate::target_fixtures::materialize_symlink_fixture(&target).expect_err("bad path");
    assert!(err.contains("path invalid"));

    write_json(&meta, &symlink_meta("links/check", "target.txt"));
    std::fs::create_dir_all(target.join("links")).expect("links");
    std::fs::write(target.join("links/check"), "occupied").expect("occupied");
    let err =
        crate::target_fixtures::materialize_symlink_fixture(&target).expect_err("occupied path");
    assert!(err.contains("already exists"));
    std::fs::remove_file(target.join("links/check")).expect("remove occupied");

    std::fs::write(target.join("target.txt"), "target").expect("target");
    let fixture = crate::target_fixtures::materialize_symlink_fixture(&target)
        .expect("valid fixture")
        .expect("fixture present");
    assert!(fixture.link.is_symlink());
    fixture.cleanup().expect("cleanup valid symlink");
    assert!(!target.join("links/check").exists());
    std::fs::remove_dir_all(target).expect("cleanup target");
}

#[test]
fn symlink_fixture_existing_and_cleanup_edges_fail_closed() {
    let target =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("target-symlink-existing");
    let meta = target.join("validation_artifacts/product-cohesion/symlink-fixture.json");
    write_json(&meta, &symlink_meta("links/check", "target.txt"));
    std::fs::create_dir_all(target.join("links")).expect("links");
    std::os::unix::fs::symlink("wrong.txt", target.join("links/check")).expect("wrong symlink");
    let err = crate::target_fixtures::materialize_symlink_fixture(&target)
        .expect_err("wrong symlink target");
    assert!(err.contains("link target mismatch"));

    std::fs::remove_file(target.join("links/check")).expect("remove wrong symlink");
    std::os::unix::fs::symlink("target.txt", target.join("links/check")).expect("right symlink");
    let fixture = crate::target_fixtures::materialize_symlink_fixture(&target)
        .expect("existing symlink")
        .expect("fixture present");
    std::fs::remove_file(&fixture.link).expect("remove before cleanup");
    fixture
        .cleanup()
        .expect("missing link cleanup is idempotent");

    std::os::unix::fs::symlink("other.txt", &fixture.link).expect("changed symlink");
    let err = fixture.cleanup().expect_err("changed cleanup target");
    assert!(err.contains("cleanup refused changed target"));
    std::fs::remove_dir_all(target).expect("cleanup target");
}

#[test]
fn symlink_fixture_rejects_malformed_meta_and_parent_file() {
    let target =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("target-symlink-bad-parent");
    let meta = target.join("validation_artifacts/product-cohesion/symlink-fixture.json");
    if let Some(parent) = meta.parent() {
        std::fs::create_dir_all(parent).expect("meta parent");
    }
    std::fs::write(&meta, "{not-json").expect("bad json");
    let err =
        crate::target_fixtures::materialize_symlink_fixture(&target).expect_err("malformed json");
    assert!(err.contains("json"));

    write_json(&meta, &symlink_meta("blocked/check", "target.txt"));
    std::fs::write(target.join("blocked"), "file parent").expect("blocked file");
    let err =
        crate::target_fixtures::materialize_symlink_fixture(&target).expect_err("parent file");
    assert!(err.contains("symlink fixture mkdir"));
    assert!(!crate::target_fixtures::normal_relative("/absolute"));
    assert!(!crate::target_fixtures::normal_relative("a/../b"));
    assert!(crate::target_fixtures::normal_relative("a/b"));
    std::fs::remove_dir_all(target).expect("cleanup target");
}

#[test]
fn target_fixture_audit_reports_materialization_and_cleanup_failures() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("target-fixture-audit-errors");
    let target = root.join("fixture");
    write_json(
        &target.join("validation_artifacts/product-cohesion/symlink-fixture.json"),
        &json!({"schema":"wrong"}),
    );
    let specs = [crate::target_fixtures::TargetSpec {
        name: "fixture.json",
        rel: "fixture",
        mode: "init",
        expected_code: 0,
        require_observability: false,
        require_product: false,
        expected_check: None,
        expected_status: None,
    }];
    let failures = crate::target_fixtures::target_capability_failures_for(&root, &[], &specs);
    assert!(failures.iter().any(|item| item.contains("expected exit 0")));

    let link = root.join("not-a-symlink");
    std::fs::create_dir_all(&link).expect("cleanup dir");
    let fixture = crate::target_fixtures::MaterializedSymlink {
        link,
        target_rel: "target.txt".into(),
    };
    let err = fixture.cleanup().expect_err("directory is not a symlink");
    assert!(err.contains("readlink cleanup failed"), "{err}");
    let failed = crate::target_fixtures::result_after_cleanup(
        Some(crate::target_fixtures::MaterializedSymlink {
            link: root.join("not-a-symlink"),
            target_rel: "target.txt".into(),
        }),
        (json!({"status":"pass"}), 0),
    );
    assert_eq!(failed.1, 1);
    assert_eq!(failed.0["status"], "fail");
    assert!(
        failed.0["error"]
            .as_str()
            .unwrap_or("")
            .contains("readlink")
    );
    std::fs::remove_dir_all(root).expect("cleanup target fixture audit errors");
}
