use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn has(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

#[test]
fn red_catalog_and_audit_outputs_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("red-catalog-output");
    let store = crate::schema_catalog::load(&root);
    let mut failures = BTreeMap::new();
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!({"bad": true}),
    );
    crate::audit::red::catalog::check(&root, &store, &mut failures);
    assert!(has(
        &failures["red-fixture-coverage"],
        "red_catalog_fixture_directory_unreadable"
    ));

    failures.clear();
    write_json(
        &root.join("fixtures/red/packet.json"),
        &json!({"expected_failure":"old"}),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([
            {
                "id":"red-one",
                "packet_path":"fixtures/red/packet.json",
                "packet_digest":crate::self_tests::boundaries::workspace_fixtures::sha('1'),
                "expected_failure":"new"
            },
            {
                "id":"red-one",
                "packet_path":"../escape.json",
                "packet_digest":crate::self_tests::boundaries::workspace_fixtures::sha('2'),
                "expected_failure":"escape"
            }
        ]),
    );
    crate::audit::red::catalog::check(&root, &store, &mut failures);
    let red_failures = &failures["red-fixture-coverage"];
    assert!(
        has(red_failures, "red_catalog_packet_contract_invalid"),
        "{red_failures:?}"
    );

    assert_eq!(
        crate::audit::package::outputs::package_status(&failures),
        "fail"
    );
    assert_eq!(
        crate::audit::package::outputs::package_status(&BTreeMap::new()),
        "pass"
    );
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let (stdout, stderr) =
        crate::audit::package::outputs::write_stdio_receipts(&receipt, "fail", 2)
            .expect("stdio receipts");
    assert!(
        std::fs::read_to_string(stdout)
            .expect("stdout")
            .contains("red=2")
    );
    assert_eq!(std::fs::read_to_string(stderr).expect("stderr"), "");
    #[cfg(unix)]
    {
        let blocked = root.join("blocked/validator-receipt.json");
        std::fs::create_dir_all(blocked.parent().expect("blocked parent")).expect("blocked dir");
        let stdout = blocked.with_extension("stdout.txt");
        std::os::unix::fs::symlink(root.join("target.txt"), &stdout).expect("stdout symlink");
        let err = crate::audit::package::outputs::write_stdio_receipts(&blocked, "fail", 1)
            .expect_err("unsafe stdout path must fail");
        assert!(err.contains("symlink"), "{err}");
    }
    std::fs::remove_dir_all(root).expect("cleanup red catalog output");
}
