#[test]
fn wrong_scope_identity_duplicates_and_registered_hidden_fail_closed() {
    let fixture = JourneyFixture::new("registry-adversarial");
    let package = fixture.build("packages/current.hugpkg");
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        None,
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    let registry_as_discovery = registry_document(&binding, true, true).unwrap();
    assert_eq!(
        observe_discovery_file(
            &mut write_scoped(
                fixture.confined(),
                "host/discovery.json",
                &registry_as_discovery,
            ),
            &binding,
            &host,
        )
        .unwrap_err()
        .id(),
        ErrorId::ProvenanceMismatch
    );

    let hidden_registry = registry_document(&binding, true, false).unwrap();
    assert_eq!(
        observe_app_registry(Some(&hidden_registry), &binding, &host)
            .unwrap()
            .verdict(),
        AppRegistryVerdict::Verified,
    );
    assert_eq!(
        observe_discovery_file(
            &mut write_scoped(
                fixture.confined(),
                "host/hidden-registry.json",
                &hidden_registry
            ),
            &binding,
            &host,
        )
        .unwrap_err()
        .id(),
        ErrorId::ProvenanceMismatch,
    );
}

#[test]
fn unsupported_app_surfaces_cannot_be_promoted_by_supplied_bytes() {
    let fixture = JourneyFixture::new("unsupported-app");
    let package = fixture.build("packages/current.hugpkg");
    let host = HostCapabilityDeclaration::unavailable_codex_app(
        &fixture.root,
        &fixture.project,
        "codex-app-api-unavailable",
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    let registry = registry_document(&binding, true, true).unwrap();
    assert_eq!(
        observe_app_registry(None, &binding, &host)
            .unwrap()
            .verdict(),
        AppRegistryVerdict::Unsupported,
    );
    assert_eq!(
        observe_discovery(None, &binding, &host)
            .unwrap()
            .discovery_verdict(),
        DiscoveryVerdict::Unsupported,
    );
    assert_eq!(
        observe_discovery(Some(&registry), &binding, &host)
            .unwrap_err()
            .id(),
        ErrorId::ProvenanceMismatch,
    );
}

#[test]
fn same_size_substitution_partial_install_and_renamed_root_preserve_external_state() {
    let fixture = JourneyFixture::new("filesystem-substitution");
    let package = fixture.build("packages/current.hugpkg");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"AAAA");
    let expected = digest(b"AAAA");
    fs::write(fixture.root.join("state/value.bin"), b"BBBB").unwrap();
    assert!(!file.apply(Some(&expected), Some(b"CCCC")).unwrap());
    assert_eq!(file.inspect(16).unwrap().unwrap(), b"BBBB");

    write_scoped(
        fixture.confined(),
        "plugins/harness-ultragoal.hugpkg",
        b"partial",
    );
    let plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        InstallScope::Personal,
        "plugins/harness-ultragoal.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    assert_eq!(
        install(&plan, &package, &mut ScopedInstall::new(fixture.confined()))
            .unwrap_err()
            .id(),
        ErrorId::InstallConflict,
    );
    assert_eq!(
        ScopedFile::new(fixture.confined(), "plugins/harness-ultragoal.hugpkg")
            .unwrap()
            .inspect(64)
            .unwrap()
            .unwrap(),
        b"partial",
    );

    let root = fixture.confined();
    let moved = renamed(&fixture.root, "renamed");
    fs::rename(&fixture.root, &moved).unwrap();
    fs::create_dir(&fixture.root).unwrap();
    assert_eq!(
        ScopedFile::new(root, "state/value.bin").unwrap_err().id(),
        ErrorId::ObjectChanged,
    );
    fs::remove_dir(&fixture.root).unwrap();
    fs::rename(&moved, &fixture.root).unwrap();
}
