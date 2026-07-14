fn confined_registry_observation(
    fixture: &Fixture,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
    install: &ultragoal::distribution::InstallSnapshot,
) -> ultragoal::distribution::AppRegistryObservation {
    let registry = registry_document(binding, true, true).unwrap();
    let caller_bytes = observe_app_registry(Some(&registry), binding, host).unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_app_registry(&caller_bytes, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "caller-owned bytes cannot mint app-registry identity authority"
    );
    fs::create_dir_all(fixture.0.join("host")).unwrap();
    fs::write(fixture.0.join("host/discovery.json"), &registry).unwrap();
    let mut registry_as_discovery = ScopedFile::new(
        ConfinedRoot::open(&fixture.0).unwrap(),
        "host/discovery.json",
    )
    .unwrap();
    assert_eq!(
        observe_discovery_file(&mut registry_as_discovery, binding, host)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "registry bytes cannot substitute for host discovery authority"
    );
    fs::create_dir_all(fixture.0.join("app")).unwrap();
    fs::write(fixture.0.join("app/registry.json"), &registry).unwrap();
    let confined = ConfinedRoot::open(&fixture.0).unwrap();
    let mut wrong_path = ScopedFile::new(confined.clone(), "registry.json").unwrap();
    assert_eq!(
        observe_registry_file(&mut wrong_path, binding, host)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );
    let substitute = fixture.0.with_file_name(format!(
        "hul-distribution-registry-substitute-{}",
        std::process::id()
    ));
    fs::create_dir_all(substitute.join("app")).unwrap();
    fs::write(substitute.join("app/registry.json"), &registry).unwrap();
    let mut wrong_root = ScopedFile::new(
        ConfinedRoot::open(&substitute).unwrap(),
        "app/registry.json",
    )
    .unwrap();
    assert_eq!(
        observe_registry_file(&mut wrong_root, binding, host)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );
    fs::remove_dir_all(substitute).unwrap();
    let mut registry_file = ScopedFile::new(confined, "app/registry.json").unwrap();
    let app_registry = observe_registry_file(&mut registry_file, binding, host).unwrap();
    let mut caller_discovery = ScopedFile::new(
        ConfinedRoot::open(&fixture.0).unwrap(),
        "host/discovery.json",
    )
    .unwrap();
    assert_eq!(
        ultragoal::distribution::observe_discovery_file(&mut caller_discovery, binding, host)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "caller-written discovery documents cannot mint discovery authority"
    );
    let mut installed = ScopedFile::new(
        ConfinedRoot::open(&fixture.0).unwrap(),
        "plugins/harness-ultragoal.hugpkg",
    )
    .unwrap();
    assert_eq!(
        observe_supported_host_discovery(&mut installed, binding, host, install, &app_registry)
            .unwrap_err()
            .id(),
        DistributionErrorId::ObjectUnavailable,
        "package-present and registry-present cannot substitute for host discovery"
    );
    app_registry
}
