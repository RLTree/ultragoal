#[test]
#[cfg(unix)]
fn typed_identity_surfaces_bind_only_verified_same_candidate_observations() {
    let (fixture, plan, archive) = candidate();
    let package = verify_package(&plan, &archive).unwrap();
    let runtime_program = fixture.0.join("runtime-probe.sh");
    let host = HostCapabilityDeclaration::isolated(
        &fixture.0.join("home"),
        &fixture.0.join("project"),
        "isolated-contract-v1",
        Some(&runtime_program),
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    let marketplace_plan = plan_codex_marketplace(
        None,
        None,
        "local-harness-plugins",
        "Local Harness Plugins",
        CodexPlugin::harness_ultragoal(),
        package.identity().clone(),
    )
    .unwrap();
    let marketplace = observe_codex_marketplace(
        marketplace_plan.replacement(),
        &marketplace_plan,
        MarketplaceScope::Personal,
    )
    .unwrap();
    assert_eq!(marketplace.plugin_id(), "harness-ultragoal");
    assert_eq!(marketplace.version(), "0.0.11");

    let install_plan = InstallPlan::new(
        CONTEXT.into(),
        CANDIDATE.into(),
        InstallScope::PersonalFixture,
        "fixture/package.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    let installed = install(&install_plan, &package, &mut Installed::default()).unwrap();

    let cache_bytes = serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.codex-cache-observation.v1",
        "context_id":CONTEXT,
        "candidate_id":CANDIDATE,
        "cache_root_id":host.home_id(),
        "entries":[{
            "marketplace":"local-harness-plugins",
            "plugin_id":"harness-ultragoal",
            "version":"0.0.11",
            "package_tree_sha256":package.identity().tree_sha256()
        }]
    }))
    .unwrap();
    let cache_expectation = CacheExpectation::new(
        CONTEXT.into(),
        CANDIDATE.into(),
        host.home_id().into(),
        "local-harness-plugins".into(),
        "harness-ultragoal".into(),
        "0.0.11".into(),
        package.identity().tree_sha256().into(),
    )
    .unwrap();
    let cache = reconcile_cache_read_only(&cache_bytes, &cache_expectation).unwrap();
    assert_eq!(cache.context_id(), CONTEXT);
    assert_eq!(cache.candidate_id(), CANDIDATE);
    assert_eq!(cache.cache_root_id(), host.home_id());
    assert_eq!(cache.marketplace(), "local-harness-plugins");
    assert_eq!(cache.plugin_id(), "harness-ultragoal");
    assert_eq!(cache.version(), "0.0.11");
    assert_eq!(
        cache.package_tree_sha256(),
        package.identity().tree_sha256()
    );

    let registry = registry_document(&binding, true, true).unwrap();
    let app_registry = observe_app_registry(Some(&registry), &binding, &host).unwrap();
    let discovery = observe_discovery(Some(&registry), &binding, &host).unwrap();
    let before = snapshot_tree(&fixture.0);
    let runtime_plan = RuntimeProbePlan::new(
        binding.clone(),
        &host,
        &runtime_program,
        Vec::new(),
        Duration::from_secs(5),
    )
    .unwrap();
    let (_, runtime) = runtime_plan.execute_bound().unwrap();

    let surfaces = [
        SurfaceIdentity::from_verified_install(installed.snapshot(), &binding).unwrap(),
        SurfaceIdentity::from_verified_cache(&cache, &binding).unwrap(),
        SurfaceIdentity::from_verified_marketplace(&marketplace, &binding).unwrap(),
        SurfaceIdentity::from_verified_app_registry(&app_registry, &binding).unwrap(),
        SurfaceIdentity::from_verified_discovery(&discovery, &binding).unwrap(),
        runtime,
    ];
    assert_eq!(
        verify_bound_surface_chain(&surfaces, &binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "verified host observations cannot substitute for package publication authority"
    );
    assert_eq!(
        snapshot_tree(&fixture.0),
        before,
        "identity joins are zero-write"
    );

    let substituted = PackageIdentity::new(
        package.identity().source().clone(),
        package.identity().tree_sha256().into(),
        SUBSTITUTE.into(),
    )
    .unwrap();
    let wrong_binding = JourneyBinding::new(substituted, &host, "local-harness-plugins").unwrap();
    for result in [
        SurfaceIdentity::from_verified_marketplace(&marketplace, &wrong_binding),
        SurfaceIdentity::from_verified_install(installed.snapshot(), &wrong_binding),
        SurfaceIdentity::from_verified_app_registry(&app_registry, &wrong_binding),
        SurfaceIdentity::from_verified_discovery(&discovery, &wrong_binding),
    ] {
        assert_eq!(
            result.unwrap_err().id(),
            DistributionErrorId::ProvenanceMismatch
        );
    }
    let substituted_tree = PackageIdentity::new(
        package.identity().source().clone(),
        WRONG_TREE.into(),
        package.identity().archive_sha256().into(),
    )
    .unwrap();
    let wrong_tree_binding =
        JourneyBinding::new(substituted_tree, &host, "local-harness-plugins").unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_cache(&cache, &wrong_tree_binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );

    cache_identity_substitution_controls(&package, &binding, &host, &cache_bytes);

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
        SurfaceIdentity::from_verified_marketplace(&unavailable, &binding)
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
    assert_eq!(wrong_version.version(), "9.9.9");
    assert_eq!(wrong_version.package_sha256(), package.package_sha256());
    assert_eq!(
        SurfaceIdentity::from_verified_marketplace(&wrong_version, &binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "same package digest cannot promote a different marketplace version"
    );

    let exact_version_catalog = serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.marketplace-catalog.v1",
        "context_id":CONTEXT,
        "candidate_id":CANDIDATE,
        "scope":"personal",
        "plugins":[{
            "plugin_id":"harness-ultragoal",
            "version":"0.0.11",
            "origin":"plugins/harness-ultragoal",
            "package_sha256":package.package_sha256()
        }]
    }))
    .unwrap();
    assert_eq!(
        verify_marketplace(&exact_version_catalog, &wrong_version_expectation)
            .unwrap_err()
            .id(),
        DistributionErrorId::InstallConflict,
        "caller-authored expectation drift cannot relabel exact catalog bytes"
    );

    let hidden_registry = registry_document(&binding, true, false).unwrap();
    let hidden = observe_discovery(Some(&hidden_registry), &binding, &host).unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_discovery(&hidden, &binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );

    let substituted_runtime = RuntimeProbePlan::new(
        binding,
        &host,
        &runtime_program,
        vec!["sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into()],
        Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(
        substituted_runtime.execute_bound().unwrap_err().id(),
        DistributionErrorId::ProvenanceMismatch
    );
}
