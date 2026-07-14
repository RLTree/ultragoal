use crate::distribution::{
    CacheExpectation, Capability, CodexPlugin, DiscoveryVerdict, ExpectedPrior, ExpectedTree,
    HostCapabilityDeclaration, HostCapabilityState, IdentitySurface, InstallPlan, InstallScope,
    JourneyBinding, MarketplaceScope, RuntimeProbePlan, RuntimeVerdict, ScopedFile, ScopedInstall,
    ScopedTree, SurfaceIdentity, apply_marketplace, execute_runtime_probe, install,
    materialize_package, observe_codex_marketplace, observe_registry_file, plan_codex_marketplace,
    reconcile_cache_file, registry_document, verify_bound_surface_chain,
};
use crate::distribution_fixture::{PLUGIN_ID, VERSION, digest};
use crate::package_journey_fixture::{JourneyFixture, write_scoped};
use crate::runtime_session::{program, valid_args};
use serde_json::json;
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
    let install = install(&install_plan, &first, &mut install_effects).unwrap();
    let installed = ScopedFile::new(fixture.confined(), "plugins/harness-ultragoal.hugpkg")
        .unwrap()
        .inspect(65 * 1024 * 1024)
        .unwrap()
        .unwrap();
    assert_eq!(installed, first.archive());

    let executable = program();
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
    let cache_bytes = serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.codex-cache-observation.v1",
        "context_id":first.context_id(), "candidate_id":first.candidate_id(),
        "cache_root_id":host.home_id(),
        "entries":[{
            "marketplace":"local-harness-plugins", "plugin_id":PLUGIN_ID,
            "version":VERSION, "package_tree_sha256":first.identity().tree_sha256()
        }]
    }))
    .unwrap();
    let mut cache_file = write_scoped(fixture.confined(), "cache/observation.json", &cache_bytes);
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
    let cache = reconcile_cache_file(&mut cache_file, &cache_expected).unwrap();

    let registry_bytes = registry_document(&binding, true, true).unwrap();
    let mut registry_file = write_scoped(fixture.confined(), "app/registry.json", &registry_bytes);
    let before_reads = fixture.tree();
    let observations = observe_registry_file(&mut registry_file, &binding, &host).unwrap();
    let app = observations.app_registry;
    let discovery = observations.discovery;
    assert_eq!(discovery.discovery_verdict(), DiscoveryVerdict::Visible);
    assert!(discovery.is_current_visible());
    assert!(app.observation_sha256().is_some());
    assert_eq!(
        fixture.tree(),
        before_reads,
        "all observation APIs are zero-write"
    );

    let runtime_plan = RuntimeProbePlan::new(
        binding.clone(),
        &host,
        &executable,
        valid_args(),
        Duration::from_secs(10),
    )
    .unwrap();
    let before_runtime = fixture.tree();
    let runtime = execute_runtime_probe(&runtime_plan).unwrap();
    assert_eq!(runtime.runtime_verdict(), RuntimeVerdict::Executed);
    assert!(runtime.is_current_execution());
    assert_eq!(
        fixture.tree(),
        before_runtime,
        "subprocess probe is zero-write in scope"
    );

    let package = first.identity().clone();
    let surfaces = vec![
        SurfaceIdentity::new(
            package.clone(),
            IdentitySurface::Marketplace,
            marketplace.catalog_sha256().unwrap().into(),
            None,
        )
        .unwrap(),
        SurfaceIdentity::new(
            package.clone(),
            IdentitySurface::Installed,
            digest(&serde_json::to_vec(install.snapshot()).unwrap()),
            Some(package.tree_sha256().into()),
        )
        .unwrap(),
        SurfaceIdentity::new(
            package.clone(),
            IdentitySurface::Cache,
            cache.observation_sha256().into(),
            Some(package.tree_sha256().into()),
        )
        .unwrap(),
        SurfaceIdentity::new(
            package.clone(),
            IdentitySurface::Discovery,
            discovery.observation_sha256().unwrap().into(),
            None,
        )
        .unwrap(),
        SurfaceIdentity::new(
            package,
            IdentitySurface::Runtime,
            runtime.output_sha256().unwrap().into(),
            None,
        )
        .unwrap(),
    ]
    .into_iter()
    .map(|row| row.bind_journey(&binding).unwrap())
    .collect::<Vec<_>>();
    verify_bound_surface_chain(&surfaces, &binding).unwrap();
}
