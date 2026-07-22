use super::*;
use crate::distribution::{
    Capability, HostCapabilityDeclaration, HostCommandPlan, JourneyBinding, ScopedFile, ScopedTree,
    ISOLATED_MARKETPLACE_NAME,
};
use crate::inventory::AuthorityCatalog;
use std::path::Path;

use super::isolated_observation::IsolatedObservation;
use super::temporary_root;

pub(super) enum IsolatedTransactionFailure {
    Message(&'static str),
    RecoveryRequired(crate::distribution::host_effect::HostLifecycleRecoveryCarrier),
}

impl From<&'static str> for IsolatedTransactionFailure {
    fn from(message: &'static str) -> Self {
        Self::Message(message)
    }
}

pub(super) fn execute(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    artifact: &ProductionPackageArtifact,
    retain_root: bool,
) -> Result<IsolatedObservation, IsolatedTransactionFailure> {
    let root_path = temporary_root::create().map_err(|_| "isolated host root unavailable")?;
    let confined = ConfinedRoot::open(&root_path).map_err(|_| "isolated host root unavailable")?;
    let mut result = execute_inner(context, catalog, artifact, confined.clone(), &root_path);
    if retain_root && result.is_ok() {
        if let Ok(observation) = &mut result {
            observation.retained_root = Some(root_path.display().to_string());
        }
        drop(confined);
        return result;
    }
    if confined.remove_owned().is_err() {
        return Err(IsolatedTransactionFailure::Message(
            "disposable isolated host cleanup failed",
        ));
    }
    result
}

fn execute_inner(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    artifact: &ProductionPackageArtifact,
    confined: ConfinedRoot,
    root_path: &Path,
) -> Result<IsolatedObservation, IsolatedTransactionFailure> {
    let mut package_tree = ScopedTree::new(confined.clone(), "plugins/harness-ultragoal")
        .map_err(|_| "package materialization target failed")?;
    let package_publication = artifact
        .materialize_marketplace_source(context, catalog, &mut package_tree)
        .map_err(|_| "package materialization failed")?;
    let marketplace_catalog = ScopedFile::new(confined.clone(), ".agents/plugins/marketplace.json")
        .map_err(|_| "isolated marketplace catalog target failed")?;
    artifact
        .materialize_marketplace_catalog(context, catalog, &marketplace_catalog)
        .map_err(|_| "isolated marketplace catalog materialization failed")?;
    let runtime_path = root_path.join("plugins/harness-ultragoal/runtime/runtime-probe-bin");
    let host = HostCapabilityDeclaration::isolated(
        root_path,
        root_path,
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
        ISOLATED_MARKETPLACE_NAME,
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
}

fn execute_bound(
    transaction: InstallTransaction<'_>,
) -> Result<IsolatedObservation, IsolatedTransactionFailure> {
    let InstallTransaction {
        context,
        catalog,
        artifact,
        confined: _confined,
        host,
        binding,
        package_tree,
        root_path,
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
        ISOLATED_MARKETPLACE_NAME,
        &root_path,
    )
    .map_err(|_| "isolated Codex command plan failed")?;
    let binding_sha256 = binding.binding_sha256().to_owned();
    let executable = crate::distribution::resolve_codex_executable()
        .map_err(|_| "pinned Codex executable unavailable")?;
    let ledger_root = isolated_ledger_root(&root_path)?;
    let result = crate::distribution::host_effect::execute_host_lifecycle_transaction(
        plan,
        package.clone(),
        command_plan,
        binding,
        host,
        &ledger_root,
        "isolated-codex-install-test".to_owned(),
        "harness-ultragoal-package-install-test".to_owned(),
        executable,
        &root_path,
        crate::distribution::host_effect::HostLifecycleObservationInput {
            marketplace_source_path: root_path.join("plugins/harness-ultragoal"),
            marketplace_source_root: root_path.to_path_buf(),
            marketplace: ISOLATED_MARKETPLACE_NAME.to_owned(),
            plugin: "harness-ultragoal".to_owned(),
        },
    )?;
    let result = resolve_transaction_outcome(result)?;
    artifact
        .verify_marketplace_source(context, catalog, &package_tree)
        .map_err(|_| "materialized package changed during transaction")?;
    let surfaces = result.surfaces;
    Ok(IsolatedObservation {
        marketplace_source_tree_sha256: artifact.snapshot().identity().tree_sha256().into(),
        installed_observation_sha256: surfaces.installed,
        cache_observation_sha256: surfaces.cache,
        marketplace_observation_sha256: surfaces.registry,
        runtime_observation_sha256: surfaces.runtime,
        journey_binding_sha256: binding_sha256,
        retained_root: None,
    })
}

fn resolve_transaction_outcome(
    mut outcome: crate::distribution::host_effect::HostLifecycleTransactionOutcome,
) -> Result<
    crate::distribution::host_effect::HostLifecycleTransactionResult,
    IsolatedTransactionFailure,
> {
    for _ in 0..3 {
        outcome = match outcome {
            crate::distribution::host_effect::HostLifecycleTransactionOutcome::Completed(
                result,
            ) => return Ok(result),
            crate::distribution::host_effect::HostLifecycleTransactionOutcome::FinalizedFailure(
                cause,
            ) => return Err(IsolatedTransactionFailure::Message(cause)),
            crate::distribution::host_effect::HostLifecycleTransactionOutcome::RecoveryRequired(
                carrier,
            ) => carrier.reobserve_and_resolve(),
        };
    }
    match outcome {
        crate::distribution::host_effect::HostLifecycleTransactionOutcome::RecoveryRequired(
            carrier,
        ) => Err(IsolatedTransactionFailure::RecoveryRequired(carrier)),
        crate::distribution::host_effect::HostLifecycleTransactionOutcome::FinalizedFailure(
            cause,
        ) => Err(IsolatedTransactionFailure::Message(cause)),
        crate::distribution::host_effect::HostLifecycleTransactionOutcome::Completed(result) => {
            Ok(result)
        }
    }
}

fn isolated_ledger_root(root: &Path) -> Result<std::path::PathBuf, &'static str> {
    let ledger = root.join("host-lifecycle-ledger");
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&ledger)
        .map_err(|_| "isolated host ledger root creation failed")?;
    Ok(ledger)
}
