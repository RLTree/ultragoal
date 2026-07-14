fn assert_catalog_substitutions(exact: CacheObservationFixture<'_>, exact_bytes: &[u8]) {
    let wrong_plugin = CacheObservationFixture {
        plugin_id: "other-plugin",
        ..exact
    };
    assert_install_conflict(&wrong_plugin.bytes(), &exact.expectation());

    let wrong_marketplace = CacheObservationFixture {
        marketplace: "other-marketplace",
        ..exact
    };
    assert_install_conflict(&wrong_marketplace.bytes(), &exact.expectation());

    let mut ambiguous: serde_json::Value = serde_json::from_slice(exact_bytes).unwrap();
    ambiguous["entries"].as_array_mut().unwrap().push(json!({
        "marketplace":"other-marketplace",
        "plugin_id":"harness-ultragoal",
        "version":"9.9.9",
        "package_tree_sha256":WRONG_TREE
    }));
    assert_install_conflict(
        &serde_json::to_vec(&ambiguous).unwrap(),
        &exact.expectation(),
    );
}

fn assert_install_conflict(bytes: &[u8], expectation: &CacheExpectation) {
    assert_eq!(
        reconcile_cache_read_only(bytes, expectation)
            .unwrap_err()
            .id(),
        DistributionErrorId::InstallConflict
    );
}
