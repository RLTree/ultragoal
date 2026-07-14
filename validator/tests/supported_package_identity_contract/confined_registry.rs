fn confined_registry_observations(
    fixture: &Fixture,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> ultragoal::distribution::RegistryObservations {
    let registry = registry_document(binding, true, true).unwrap();
    let caller_bytes = observe_app_registry(Some(&registry), binding, host).unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_app_registry(&caller_bytes, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "caller-owned bytes cannot mint app-registry identity authority"
    );
    let caller_discovery = observe_discovery(Some(&registry), binding, host).unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_discovery(&caller_discovery, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "caller-owned bytes cannot mint discovery identity authority"
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
    observe_registry_file(&mut registry_file, binding, host).unwrap()
}
