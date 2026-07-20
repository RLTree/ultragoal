use super::*;
use crate::distribution::{
    Capability, HostCapabilityDeclaration, HostCommandPlan, JourneyBinding, ScopedTree,
};
use crate::inventory::AuthorityCatalog;
use std::fs;
use std::path::Path;

use super::isolated_observation::IsolatedObservation;
use super::temporary_root;

pub(super) fn execute(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    artifact: &ProductionPackageArtifact,
) -> Result<IsolatedObservation, &'static str> {
    let root_path = temporary_root::create()?;
    let confined = ConfinedRoot::open(&root_path).map_err(|_| "isolated host root unavailable")?;
    let result = execute_inner(context, catalog, artifact, confined.clone(), &root_path);
    if confined.remove_owned().is_err() {
        return Err("disposable isolated host cleanup failed");
    }
    result
}

fn execute_inner(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    artifact: &ProductionPackageArtifact,
    confined: ConfinedRoot,
    root_path: &Path,
) -> Result<IsolatedObservation, &'static str> {
    let mut package_tree = ScopedTree::new(confined.clone(), "plugins/harness-ultragoal")
        .map_err(|_| "package materialization target failed")?;
    let package_publication = artifact
        .materialize_marketplace_source(context, catalog, &mut package_tree)
        .map_err(|_| "package materialization failed")?;
    let runtime_path = root_path.join("plugins/harness-ultragoal/runtime/runtime-probe-bin");
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
    if package_publication.root_id() != binding.home_id()
        || package_publication.tree_sha256() != artifact.snapshot().identity().tree_sha256()
    {
        return Err("materialized package identity diverged");
    }
    execute_bound(InstallTransaction {
        context,
        catalog,
        artifact,
        confined,
        host,
        binding,
        package_tree,
        root_path: root_path.to_path_buf(),
        runtime_path,
    })
}

struct InstallTransaction<'a> {
    context: &'a LiveContext,
    catalog: &'a AuthorityCatalog,
    artifact: &'a ProductionPackageArtifact,
    confined: ConfinedRoot,
    host: HostCapabilityDeclaration,
    binding: JourneyBinding,
    package_tree: ScopedTree,
    root_path: std::path::PathBuf,
    runtime_path: std::path::PathBuf,
}

fn execute_bound(transaction: InstallTransaction<'_>) -> Result<IsolatedObservation, &'static str> {
    let InstallTransaction {
        context,
        catalog,
        artifact,
        confined: _confined,
        host,
        binding,
        package_tree,
        root_path,
        runtime_path,
    } = transaction;
    let package = artifact.snapshot().identity().clone();
    let authority = crate::plugin_product::lifecycle::PackageAuthority {
        version: crate::plugin_product::lifecycle::Version::parse(package.source().version())
            .map_err(|_| "package version is lifecycle-valid")?,
        package_sha256: package.archive_sha256().to_owned(),
        inventory_sha256: package.source().accepted_inventory_sha256().to_owned(),
        candidate_id: package.source().candidate_id().to_owned(),
    };
    let plan = crate::plugin_product::lifecycle::plan(
        &crate::plugin_product::lifecycle::LifecycleState::default(),
        &crate::plugin_product::lifecycle::LifecycleRequest {
            intent: crate::plugin_product::lifecycle::LifecycleIntent::FreshInstall,
            target: Some(authority),
            prior_authority: None,
            authorization: crate::plugin_product::lifecycle::LifecycleAuthorization {
                allow_host_write: true,
                allow_downgrade: false,
                expected_installed_sha256: None,
            },
        },
    )
    .map_err(|_| "lifecycle plan failed")?;
    let command_plan = HostCommandPlan::repository_install_in_isolated_codex_home(
        &package,
        root_path.to_str().ok_or("isolated root path is not utf8")?,
        "local-harness-plugins",
        &root_path,
    )
    .map_err(|_| "isolated Codex command plan failed")?;
    let binding_sha256 = binding.binding_sha256().to_owned();
    let executable = codex_executable()?;
    let result = crate::distribution::host_effect::execute_host_lifecycle_transaction(
        plan,
        package.clone(),
        command_plan,
        binding,
        host,
        &root_path,
        "isolated-codex-install-test".to_owned(),
        "harness-ultragoal-package-install-test".to_owned(),
        &executable,
        &root_path,
        crate::distribution::host_effect::HostLifecycleObservationInput {
            installed_path: root_path.join("plugins/harness-ultragoal"),
            cache_path: root_path.join("plugins/cache/local-harness-plugins/harness-ultragoal"),
            runtime_path,
            marketplace: "local-harness-plugins".to_owned(),
            plugin: "harness-ultragoal".to_owned(),
        },
    )?;
    artifact
        .verify_marketplace_source(context, catalog, &package_tree)
        .map_err(|_| "materialized package changed during transaction")?;
    let surfaces = result.surfaces;
    Ok(IsolatedObservation {
        marketplace_source_tree_sha256: artifact.snapshot().identity().tree_sha256().into(),
        installed_observation_sha256: surfaces.installed,
        cache_observation_sha256: surfaces.cache,
        marketplace_observation_sha256: surfaces.registry,
        app_registry_observation_sha256: surfaces.discovery,
        runtime_observation_sha256: surfaces.runtime,
        journey_binding_sha256: binding_sha256,
    })
}

fn codex_executable() -> Result<std::path::PathBuf, &'static str> {
    let candidates = [
        std::path::PathBuf::from("/opt/homebrew/bin/codex"),
        std::path::PathBuf::from("/usr/local/bin/codex"),
        std::path::PathBuf::from("/usr/bin/codex"),
    ]
    .into_iter()
    .filter_map(|path| path.canonicalize().ok())
    .find(|path| path.is_file());
    candidates.ok_or("pinned Codex executable unavailable")
}
