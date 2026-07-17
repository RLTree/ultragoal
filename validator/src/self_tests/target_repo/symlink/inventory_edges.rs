use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn symlink_fixture_observability_and_inventory_edges() {
    let target = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "target_boundary-symlink-long",
    );
    write_json(
        &target.join("validation_artifacts/product-cohesion/symlink-fixture.json"),
        &json!({
            "schema": "harness-ultragoal.target-fixture-symlink.v1",
            "link_path": "x".repeat(300),
            "target_path": "target.txt"
        }),
    );
    let err = crate::target_fixtures::materialize_symlink_fixture(&target)
        .expect_err("overlong link name fails symlink creation");
    assert!(err.contains("symlink fixture materialize failed"), "{err}");
    std::fs::remove_dir_all(target).expect("cleanup symlink");

    let mut failures = Vec::new();
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("target_boundary-lane-size");
    crate::claim_semantics::lane::policy::check_lanes(
        &json!({}),
        &json!({"lanes":[{
            "id": "under-work-units",
            "lane_size_evidence": {
                "owned_path_groups": ["validator", "fixtures", "docs"],
                "work_units": ["schema"],
                "independent_outcome": "",
                "why_not_parent_inline": "",
                "why_not_smaller": ""
            }
        }]}),
        &json!({}),
        &[],
        &root,
        0,
        &mut failures,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.error == "lane_too_small_or_overlapping")
    );

    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let target_repo = repo.join("fixtures/target-repo/valid-observability");
    let (receipt, _) = crate::target_repo::audit_target_repo(
        &target_repo,
        "fresh-init",
        "observability positive",
        &[],
        true,
        false,
        None,
    );
    assert_eq!(receipt["checks"]["observability-stack"]["status"], "pass");
    assert_eq!(
        receipt["checks"]["observability-stack"]["detail"],
        "agent observability event ledger and query surface complete"
    );

    let inventory = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "target_boundary-unreadable-inventory",
    );
    std::fs::create_dir_all(inventory.join("denied")).expect("denied dir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            &inventory.join("denied"),
            std::fs::Permissions::from_mode(0o000),
        )
        .expect("deny permissions");
        let err = crate::package::inventory::closure::actual_files(&inventory)
            .expect_err("unreadable directory fails closed");
        assert!(err.contains("walk failed"), "{err}");
        std::fs::set_permissions(
            &inventory.join("denied"),
            std::fs::Permissions::from_mode(0o700),
        )
        .expect("restore permissions");
    }
    std::fs::remove_dir_all(inventory).expect("cleanup inventory");
    let _ = std::fs::remove_dir_all(root);
}
