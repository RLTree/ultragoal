#[test]
fn coverage_runner_uses_product_package_digest_command_surface() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let runner =
        std::fs::read_to_string(root.join(".harness/run-coverage.sh")).expect("coverage runner");
    assert!(
        runner.contains("target/debug/ultragoal --root . package digest"),
        "coverage runner must call the canonical product CLI package digest command"
    );
    assert!(
        runner.contains(" package digest"),
        "coverage runner must call the product package digest command"
    );
    assert!(
        !runner.contains("cargo run"),
        "coverage runner must not compute claim-bound package digest through a cargo run proxy"
    );
    assert!(
        runner.contains("CARGO_TARGET_DIR=\"$coverage_target_dir\"")
            && runner.contains("coverage_target_dir"),
        "coverage runner must isolate llvm-cov artifacts from the hot-loop target directory"
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
