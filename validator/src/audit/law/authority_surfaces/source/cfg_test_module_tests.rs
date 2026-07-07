#[test]
fn cfg_test_sibling_detection_fails_closed_for_missing_or_rootless_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("source-cfg-test-sibling");
    std::fs::create_dir_all(&root).expect("root");

    assert!(!super::cfg_test_path_sibling_file(
        &root,
        "validator.rs",
        "validator"
    ));
    assert!(!super::cfg_test_path_sibling_file(
        &root,
        "validator/src/missing/module.rs",
        "module"
    ));

    std::fs::remove_dir_all(root).expect("cleanup source cfg sibling");
}

#[test]
fn cfg_test_path_module_detection_preserves_attribute_decorated_test_modules() {
    let text = r#"
        #[path = "status/success.rs"]
        #[allow(dead_code)]
        mod success;
    "#;

    assert!(super::declares_path_module(
        text,
        "status/success.rs",
        "success"
    ));
}
