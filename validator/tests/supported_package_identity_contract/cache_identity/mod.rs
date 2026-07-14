include!("observation_fixture.rs");

include!("package_binding.rs");

include!("catalog_rejection.rs");

include!("journey_binding.rs");

fn cache_identity_substitution_controls(
    package: &ultragoal::distribution::PackageSnapshot,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
    exact_bytes: &[u8],
) {
    let exact = CacheObservationFixture::exact(host, package.identity().tree_sha256());
    assert_package_binding_substitutions(binding, exact);
    assert_catalog_substitutions(exact, exact_bytes);
    assert_journey_binding_substitutions(package, binding, exact, exact_bytes);
}
