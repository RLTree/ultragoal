#[test]
fn coverage_runner_is_a_thin_typed_product_adapter() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let runner =
        std::fs::read_to_string(root.join(".harness/run-coverage.sh")).expect("coverage runner");
    assert!(runner.contains("target/debug/ultragoal --root . coverage prove"));
    assert!(runner.contains("--mode strict"));
    assert!(
        !runner.contains("cargo run"),
        "coverage runner must invoke the built product binary"
    );
    assert!(
        !runner.contains("python3 -") && !runner.contains("<<'PY'"),
        "coverage policy and receipt projection belong to the typed Rust boundary"
    );
    assert!(
        !runner.contains("ultragoal-validator"),
        "coverage runner must not mint claim-bound receipts through the compatibility binary"
    );
    assert!(
        !runner.contains(" package digest"),
        "the shell adapter must not mint candidate identity"
    );
}
