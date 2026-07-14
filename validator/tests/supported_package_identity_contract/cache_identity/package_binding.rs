fn assert_package_binding_substitutions(
    binding: &JourneyBinding,
    exact: CacheObservationFixture<'_>,
) {
    let wrong_version = CacheObservationFixture {
        version: "9.9.9",
        ..exact
    }
    .reconcile();
    assert_eq!(wrong_version.version(), "9.9.9");
    assert_provenance_mismatch(
        SurfaceIdentity::from_verified_cache(&wrong_version, binding),
        "an exact 0.0.11 package cannot promote a same-tree 9.9.9 cache",
    );

    let wrong_root = CacheObservationFixture {
        cache_root_id: WRONG_HOME,
        ..exact
    }
    .reconcile();
    assert_eq!(wrong_root.cache_root_id(), WRONG_HOME);
    assert_provenance_mismatch(
        SurfaceIdentity::from_verified_cache(&wrong_root, binding),
        "a caller-authored cache root cannot substitute for the journey home",
    );

    let wrong_tree = CacheObservationFixture {
        package_tree_sha256: WRONG_TREE,
        ..exact
    }
    .reconcile();
    assert_provenance_mismatch(
        SurfaceIdentity::from_verified_cache(&wrong_tree, binding),
        "same-path cache tree substitution cannot promote",
    );

    for substituted in [
        CacheObservationFixture {
            context_id: SUBSTITUTE,
            ..exact
        },
        CacheObservationFixture {
            candidate_id: SUBSTITUTE,
            ..exact
        },
    ] {
        assert_provenance_mismatch(
            SurfaceIdentity::from_verified_cache(&substituted.reconcile(), binding),
            "context and candidate substitution must fail at package promotion",
        );
    }
}

fn assert_provenance_mismatch(
    result: Result<SurfaceIdentity, ultragoal::distribution::DistributionError>,
    message: &str,
) {
    assert_eq!(
        result.unwrap_err().id(),
        DistributionErrorId::ProvenanceMismatch,
        "{message}"
    );
}
