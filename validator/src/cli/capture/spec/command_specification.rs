use super::CommandSpec;

#[test]
fn test_only_catalog_helper_reaches_the_private_binding_leaf() {
    let spec = CommandSpec::catalog_read("capture-test-control", "true");
    assert!(spec.require_catalog_binding().is_ok());
}
