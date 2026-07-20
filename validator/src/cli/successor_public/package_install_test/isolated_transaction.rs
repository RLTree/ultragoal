use super::*;
use crate::distribution::{
    CacheExpectation, Capability, CodexPlugin, ExpectedPrior, ExpectedTree,
    HostCapabilityDeclaration, InstallPlan, InstallScope, InstalledPackageRuntimeProbeRequest,
    JourneyBinding, MarketplaceScope, RuntimeProbePlan, ScopedInstall, ScopedTree, SurfaceIdentity,
    apply_marketplace, install, observe_codex_marketplace, observe_registry_file,
    plan_codex_marketplace, publish_cache_file, publish_installed_runtime_probe,
    publish_registry_file, reconcile_cache_file,
};
use crate::inventory::AuthorityCatalog;
use std::fs;
use std::time::Duration;

use super::isolated_observation::IsolatedObservation;
use super::temporary_root;

pub(super) fn execute(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    artifact: &ProductionPackageArtifact,
) -> Result<IsolatedObservation, &'static str> {
    let root_path = temporary_root::create()?;
    let result = execute_inner(context, catalog, artifact, &root_path);
    if fs::remove_dir_all(&root_path).is_err() {
        return Err("disposable isolated host cleanup failed");
    }
    result
}

fn execute_inner(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    artifact: &ProductionPackageArtifact,
    root_path: &Path,
) -> Result<IsolatedObservation, &'static str> {
    let confined = ConfinedRoot::open(root_path).map_err(|_| "isolated host root unavailable")?;
    let runtime_file = ScopedFile::new(confined.clone(), "runtime/runtime-probe-bin")
        .map_err(|_| "runtime target unavailable")?;
    publish_installed_runtime_probe(artifact.snapshot(), &runtime_file)
        .map_err(|_| "runtime payload publication failed")?;
    let runtime_path = root_path.join("runtime/runtime-probe-bin");
    let project = root_path.join("project");
    fs::create_dir(&project).map_err(|_| "isolated project creation failed")?;
    let host = HostCapabilityDeclaration::isolated(
        root_path,
        &project,
        "isolated-codex-install-test-v1",
        Some(&runtime_path),
    )
    .map_err(|_| "isolated host capability binding failed")?;
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
        if host.state(capability) != crate::distribution::HostCapabilityState::Supported {
            return Err("isolated host capability is unavailable");
        }
    }
    let binding = JourneyBinding::new(
        artifact.snapshot().identity().clone(),
        &host,
        "local-harness-plugins",
    )
    .map_err(|_| "journey binding failed")?;
    execute_bound(
        context,
        catalog,
        artifact,
        confined,
        host,
        binding,
        runtime_path,
    )
}

fn execute_bound(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    artifact: &ProductionPackageArtifact,
    confined: ConfinedRoot,
    host: HostCapabilityDeclaration,
    binding: JourneyBinding,
    runtime_path: std::path::PathBuf,
) -> Result<IsolatedObservation, &'static str> {
    let mut package_tree =
        ScopedTree::new(confined.clone(), "repository/packages/harness-ultragoal")
            .map_err(|_| "package materialization target failed")?;
    let package_publication = artifact
        .publish(
            context,
            catalog,
            &binding,
            &ExpectedTree::Absent,
            &mut package_tree,
        )
        .map_err(|_| "package materialization failed")?;
    let package_surface = SurfaceIdentity::from_published_package(
        artifact.snapshot(),
        &package_publication,
        &binding,
    )
    .map_err(|_| "package identity observation failed")?;

    let marketplace_plan = plan_codex_marketplace(
        None,
        None,
        "local-harness-plugins",
        "Local Harness Plugins",
        CodexPlugin::harness_ultragoal(),
        artifact.snapshot().identity().clone(),
    )
    .map_err(|_| "marketplace plan failed")?;
    let mut marketplace_file = ScopedFile::new(
        confined.clone(),
        "marketplace/.codex-plugin/marketplace.json",
    )
    .map_err(|_| "marketplace target failed")?;
    apply_marketplace(&marketplace_plan, &mut marketplace_file)
        .map_err(|_| "marketplace publication failed")?;
    let marketplace_bytes = marketplace_file
        .inspect(1024 * 1024)
        .map_err(|_| "marketplace observation failed")?
        .ok_or("marketplace observation unavailable")?;
    let marketplace = observe_codex_marketplace(
        &marketplace_bytes,
        &marketplace_plan,
        MarketplaceScope::Personal,
    )
    .map_err(|_| "marketplace verification failed")?;
    let marketplace_surface = SurfaceIdentity::from_verified_marketplace(&marketplace, &binding)
        .map_err(|_| "marketplace identity failed")?;

    let install_plan = InstallPlan::new(
        artifact.context_id().into(),
        artifact.candidate_id().into(),
        InstallScope::Personal,
        "plugins/harness-ultragoal.hugpkg".into(),
        artifact.snapshot().package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .map_err(|_| "install plan failed")?;
    let mut install_effects = ScopedInstall::new(confined.clone());
    let mut installed = install(&install_plan, artifact.snapshot(), &mut install_effects)
        .map_err(|_| "install failed")?;
    installed
        .bind_journey(&binding)
        .map_err(|_| "install journey binding failed")?;
    let install_surface = SurfaceIdentity::from_verified_install(
        installed.snapshot(),
        &binding,
        &mut ScopedInstall::new(confined.clone()),
    )
    .map_err(|_| "installed identity failed")?;

    let cache_file = ScopedFile::new(confined.clone(), "cache/observation.json")
        .map_err(|_| "cache target failed")?;
    let source = artifact.snapshot().identity().source();
    let cache_expected = CacheExpectation::new(
        artifact.context_id().into(),
        artifact.candidate_id().into(),
        binding.home_id().into(),
        "local-harness-plugins".into(),
        source.plugin_id().into(),
        source.version().into(),
        artifact.snapshot().identity().tree_sha256().into(),
    )
    .map_err(|_| "cache expectation failed")?;
    publish_cache_file(&cache_file, &cache_expected).map_err(|_| "cache publication failed")?;
    let cache = reconcile_cache_file(&mut cache_file.clone(), &cache_expected)
        .map_err(|_| "cache observation failed")?;
    let cache_surface = SurfaceIdentity::from_verified_cache(&cache, &binding)
        .map_err(|_| "cache identity failed")?;

    let mut registry_file = ScopedFile::new(confined.clone(), "app/registry.json")
        .map_err(|_| "registry target failed")?;
    publish_registry_file(&mut registry_file, &binding, &host, true, true)
        .map_err(|_| "app registry publication failed")?;
    let app = observe_registry_file(&mut registry_file, &binding, &host)
        .map_err(|_| "app registry observation failed")?;
    let app_surface = SurfaceIdentity::from_verified_app_registry(&app, &binding)
        .map_err(|_| "app registry identity failed")?;

    let runtime_plan =
        RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
            binding: binding.clone(),
            host: &host,
            install: installed.snapshot(),
            effects: &mut ScopedInstall::new(confined),
            package: artifact.snapshot(),
            program: &runtime_path,
            argv: vec!["valid".into()],
            timeout: Duration::from_secs(10),
        })
        .map_err(|_| "runtime plan failed")?;
    let (runtime, runtime_surface) = runtime_plan
        .execute_bound()
        .map_err(|_| "runtime execution failed")?;
    if !runtime.is_current_execution() {
        return Err("runtime execution was not current");
    }
    let surfaces = [
        package_surface,
        install_surface,
        cache_surface,
        marketplace_surface,
        app_surface,
        runtime_surface,
    ];
    if surfaces.iter().any(|surface| {
        surface.package() != binding.package()
            || surface.journey_binding_sha256() != Some(binding.binding_sha256())
    }) {
        return Err("isolated identity surfaces diverged");
    }
    Ok(IsolatedObservation {
        installed_tree_sha256: artifact.snapshot().identity().tree_sha256().into(),
        cache_observation_sha256: cache.observation_sha256().into(),
        marketplace_observation_sha256: marketplace
            .catalog_sha256()
            .ok_or("marketplace digest unavailable")?
            .into(),
        app_registry_observation_sha256: app
            .observation_sha256()
            .ok_or("app registry digest unavailable")?
            .into(),
        runtime_observation_sha256: runtime
            .output_sha256()
            .ok_or("runtime digest unavailable")?
            .into(),
        journey_binding_sha256: binding.binding_sha256().into(),
    })
}
