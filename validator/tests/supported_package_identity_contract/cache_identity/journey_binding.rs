fn assert_journey_binding_substitutions(
    package: &ultragoal::distribution::PackageSnapshot,
    binding: &JourneyBinding,
    exact: CacheObservationFixture<'_>,
    exact_bytes: &[u8],
) {
    let wrong_marketplace = CacheObservationFixture {
        marketplace: "attacker-marketplace",
        ..exact
    }
    .reconcile();
    assert_provenance_mismatch(
        SurfaceIdentity::from_verified_cache(&wrong_marketplace, binding),
        "a self-consistent caller marketplace cannot substitute for journey authority",
    );

    let other_home = Fixture::new("other-home");
    let other_host = HostCapabilityDeclaration::isolated(
        &other_home.0.join("home"),
        &other_home.0.join("project"),
        "isolated-contract-v1",
        None,
    )
    .unwrap();
    let other_binding = JourneyBinding::new(
        package.identity().clone(),
        &other_host,
        "local-harness-plugins",
    )
    .unwrap();
    let verified = reconcile_cache_read_only(exact_bytes, &exact.expectation()).unwrap();
    assert_provenance_mismatch(
        SurfaceIdentity::from_verified_cache(&verified, &other_binding),
        "journey substitution changes the authoritative home",
    );
}
