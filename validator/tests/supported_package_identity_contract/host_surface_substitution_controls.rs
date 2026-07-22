#[cfg(unix)]
fn host_surface_substitution_controls(
    fixture: &Fixture,
    package: &PackageSnapshot,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
    installed: &InstallTransaction,
) {
    let unavailable = ultragoal::distribution::MarketplaceExpectation::new(
        CONTEXT.into(),
        CANDIDATE.into(),
        MarketplaceScope::Personal,
        "harness-ultragoal".into(),
        "0.0.11".into(),
        "plugins/harness-ultragoal".into(),
        package.package_sha256().into(),
    )
    .and_then(|expectation| unavailable_marketplace(&expectation, "host-unavailable"))
    .unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_marketplace(&unavailable, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );

    let wrong_version_expectation = ultragoal::distribution::MarketplaceExpectation::new(
        CONTEXT.into(),
        CANDIDATE.into(),
        MarketplaceScope::Personal,
        "harness-ultragoal".into(),
        "9.9.9".into(),
        "plugins/harness-ultragoal".into(),
        package.package_sha256().into(),
    )
    .unwrap();
    let wrong_version_catalog = serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.marketplace-catalog.v1",
        "context_id":CONTEXT,
        "candidate_id":CANDIDATE,
        "scope":"personal",
        "plugins":[{
            "plugin_id":"harness-ultragoal",
            "version":"9.9.9",
            "origin":"plugins/harness-ultragoal",
            "package_sha256":package.package_sha256()
        }]
    }))
    .unwrap();
    let wrong_version = verify_marketplace(&wrong_version_catalog, &wrong_version_expectation)
        .expect("wrong-version catalog is internally exact before package-source binding");
    assert_eq!(
        SurfaceIdentity::from_verified_marketplace(&wrong_version, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );

    let caller_discovery = registry_document(binding, true, true).unwrap();
    fs::create_dir_all(fixture.0.join("host")).unwrap();
    fs::write(fixture.0.join("host/discovery.json"), &caller_discovery).unwrap();
    let mut hidden_file = ScopedFile::new(
        ConfinedRoot::open(&fixture.0).unwrap(),
        "host/discovery.json",
    )
    .unwrap();
    assert_eq!(
        observe_discovery_file(&mut hidden_file, binding, host)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );

    let mut effects = ScopedInstall::new(ConfinedRoot::open(&fixture.0).unwrap());
    let substituted_runtime = RuntimeProbePlan::from_installed_package(
        ultragoal::distribution::InstalledPackageRuntimeProbeRequest {
            binding: binding.clone(),
            host,
            install: installed.snapshot(),
            effects: &mut effects,
            package,
            program: &fixture
                .0
                .join("plugins/harness-ultragoal/runtime/ultragoal-substitute"),
            timeout: Duration::from_secs(5),
        },
    );
    assert_eq!(
        substituted_runtime.unwrap_err().id(),
        DistributionErrorId::CapabilityMismatch
    );
}
