#[test]
fn claimed_catalog_scope_rolls_back_setup_failures_and_refuses_substitution() {
    super::catalog_fixture::verify_catalog_scope_construction();
}
