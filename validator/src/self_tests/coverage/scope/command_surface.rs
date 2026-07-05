#[test]
fn coverage_runner_uses_product_package_digest_command_surface() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let runner =
        std::fs::read_to_string(root.join(".harness/run-coverage.sh")).expect("coverage runner");
    assert!(
        runner.contains("\"ultragoal\""),
        "coverage runner must call the product CLI binary"
    );
    assert!(
        runner.contains("\"package\"") && runner.contains("\"digest\""),
        "coverage runner must call the product package digest command"
    );
    assert!(
        !runner.contains("ultragoal-validator"),
        "coverage runner must not mint claim-bound receipts through the compatibility binary"
    );
    assert!(
        !runner.contains("\"package-digest\""),
        "coverage runner must not mint claim-bound receipts through the compatibility command"
    );
}
