use crate::distribution::{
    CacheExpectation, Capability, CodexPlugin, DistributionErrorId, ExpectedPrior, ExpectedTree,
    HostCapabilityDeclaration, HostCapabilityState, IdentitySurface, InstallPlan, InstallScope,
    InstalledPackageRuntimeProbeRequest, JourneyBinding, MarketplaceScope, RuntimeProbePlan,
    ScopedFile, ScopedInstall, ScopedTree, SurfaceIdentity, apply_marketplace, install,
    materialize_package, observe_codex_marketplace, observe_discovery_file, observe_registry_file,
    plan_codex_marketplace, publish_cache_file, publish_discovery_file,
    publish_installed_runtime_probe, publish_registry_file, reconcile_cache_file,
    verify_bound_surface_chain,
};
use crate::distribution_fixture::{PLUGIN_ID, VERSION};
use crate::package_journey_fixture::JourneyFixture;
use std::time::Duration;

#[test]
fn clean_isolated_package_marketplace_install_discovery_runtime_journey() {
    let fixture = JourneyFixture::new("clean-product-journey");
    let plan = fixture.plan();
    let first = fixture.build("packages/first.hugpkg");
    let second = fixture.build("packages/second.hugpkg");
    assert_eq!(first.archive(), second.archive());
    assert_eq!(first.inventory(), second.inventory());

    let mut materialized =
        ScopedTree::new(fixture.confined(), "marketplace/plugins/harness-ultragoal").unwrap();
    let materialization =
        materialize_package(&plan, &ExpectedTree::Absent, &mut materialized).unwrap();
    assert_eq!(
        materialization.tree_sha256(),
        first.identity().tree_sha256()
    );

    let marketplace_plan = plan_codex_marketplace(
        None,
        None,
        "local-harness-plugins",
        "Local Harness Plugins",
        CodexPlugin::harness_ultragoal(),
        first.identity().clone(),
    )
    .unwrap();
    let mut marketplace_file = ScopedFile::new(
        fixture.confined(),
        "marketplace/.codex-plugin/marketplace.json",
    )
    .unwrap();
    apply_marketplace(&marketplace_plan, &mut marketplace_file).unwrap();
    let marketplace_bytes = marketplace_file.inspect(1024 * 1024).unwrap().unwrap();
    let marketplace = observe_codex_marketplace(
        &marketplace_bytes,
        &marketplace_plan,
        MarketplaceScope::Personal,
    )
    .unwrap();

    let install_plan = InstallPlan::new(
        first.context_id().into(),
        first.candidate_id().into(),
        InstallScope::Personal,
        "plugins/harness-ultragoal.hugpkg".into(),
        first.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    let mut install_effects = ScopedInstall::new(fixture.confined());
    let mut install = install(&install_plan, &first, &mut install_effects).unwrap();
    let installed = ScopedFile::new(fixture.confined(), "plugins/harness-ultragoal.hugpkg")
        .unwrap()
        .inspect(65 * 1024 * 1024)
        .unwrap()
        .unwrap();
    assert_eq!(installed, first.archive());

    let executable_file = ScopedFile::new(
        fixture.confined(),
        "plugins/harness-ultragoal/runtime/ultragoal",
    )
    .unwrap();
    publish_installed_runtime_probe(&first, &executable_file).unwrap();
    let executable = fixture
        .root
        .join("plugins/harness-ultragoal/runtime/ultragoal");
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&executable),
    )
    .unwrap();
    for capability in [
        Capability::Filesystem,
        Capability::ArchiveWriter,
        Capability::Install,
        Capability::Cache,
        Capability::Marketplace,
        Capability::AppRegistry,
        Capability::Discovery,
        Capability::Runtime,
    ] {
        assert_eq!(host.state(capability), HostCapabilityState::Supported);
    }
    assert_eq!(
        host.state(Capability::PluginsUi),
        HostCapabilityState::Unsupported,
    );
    let binding =
        JourneyBinding::new(first.identity().clone(), &host, "local-harness-plugins").unwrap();
    install.bind_journey(&binding).unwrap();
    let cache_file = ScopedFile::new(fixture.confined(), "cache/observation.json").unwrap();
    let cache_expected = CacheExpectation::new(
        first.context_id().into(),
        first.candidate_id().into(),
        host.home_id().into(),
        "local-harness-plugins".into(),
        PLUGIN_ID.into(),
        VERSION.into(),
        first.identity().tree_sha256().into(),
    )
    .unwrap();
    publish_cache_file(&cache_file, &cache_expected).unwrap();
    let cache = reconcile_cache_file(&mut cache_file.clone(), &cache_expected).unwrap();

    let mut registry_file = ScopedFile::new(fixture.confined(), "app/registry.json").unwrap();
    publish_registry_file(&mut registry_file, &binding, &host, true, true).unwrap();
    let app = observe_registry_file(&mut registry_file, &binding, &host).unwrap();
    let discovery_file = ScopedFile::new(fixture.confined(), "host/discovery.json").unwrap();
    publish_discovery_file(&discovery_file, &binding, &host).unwrap();
    let before_reads = fixture.tree();
    assert_eq!(
        observe_discovery_file(&discovery_file, &binding, &host)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "writer output cannot substitute for independent host discovery"
    );
    assert!(app.observation_sha256().is_some());
    assert_eq!(
        fixture.tree(),
        before_reads,
        "all observation APIs are zero-write"
    );

    let runtime_plan =
        RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
            binding: binding.clone(),
            host: &host,
            install: install.snapshot(),
            effects: &mut ScopedInstall::new(fixture.confined()),
            package: &first,
            program: &executable,
            timeout: Duration::from_secs(10),
        })
        .unwrap();
    let before_runtime = fixture.tree();
    assert_eq!(
        runtime_plan.execute_bound().unwrap_err().id(),
        DistributionErrorId::CapabilityMismatch,
        "package bytes cannot execute without confined effect authority"
    );
    assert_eq!(
        fixture.tree(),
        before_runtime,
        "refused runtime probe is zero-write in scope"
    );

    let observed_surfaces = [
        SurfaceIdentity::from_verified_install(
            install.snapshot(),
            &binding,
            &mut ScopedInstall::new(fixture.confined()),
        )
        .unwrap(),
        SurfaceIdentity::from_verified_cache(&cache, &binding).unwrap(),
        SurfaceIdentity::from_verified_marketplace(&marketplace, &binding).unwrap(),
        SurfaceIdentity::from_verified_app_registry(&app, &binding).unwrap(),
    ];
    assert_eq!(
        observed_surfaces.each_ref().map(|row| row.surface()),
        [
            IdentitySurface::Installed,
            IdentitySurface::Cache,
            IdentitySurface::Marketplace,
            IdentitySurface::AppRegistry,
        ]
    );
    assert_eq!(
        verify_bound_surface_chain(&observed_surfaces, &binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "a journey without the separately published package row cannot pass"
    );
}
