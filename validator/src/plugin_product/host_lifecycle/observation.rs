use super::transaction::InstallTestError;
use crate::distribution::{
    CacheExpectation, ConfinedRoot, ExpectedTree, PackagePlan, PackageSnapshot, ScopedFile,
    ScopedTree, materialize_package, reconcile_cache_file,
};
use crate::plugin_product::lifecycle::{PackageAuthority, Version};

const PACKAGE_LIMIT: usize = 65 * 1024 * 1024;

pub(super) fn package_authority(
    plan: &PackagePlan,
    package: &PackageSnapshot,
) -> Result<PackageAuthority, InstallTestError> {
    Ok(PackageAuthority {
        version: Version::parse(plan.version()).map_err(|_| InstallTestError::Package)?,
        package_sha256: package.package_sha256().into(),
        inventory_sha256: package.inventory_sha256().into(),
        candidate_id: package.candidate_id().into(),
    })
}

pub(super) fn materialize(
    plan: &PackagePlan,
    root: ConfinedRoot,
) -> Result<String, InstallTestError> {
    let mut tree = ScopedTree::new(root, "cache/plugins/harness-ultragoal")
        .map_err(|_| InstallTestError::Effect)?;
    materialize_package(plan, &ExpectedTree::Absent, &mut tree)
        .map(|transaction| transaction.tree_sha256().to_owned())
        .map_err(|_| InstallTestError::Effect)
}

pub(super) fn observe_installed(
    root: ConfinedRoot,
    package: &PackageSnapshot,
) -> Result<(), InstallTestError> {
    let file = ScopedFile::new(root, "plugins/harness-ultragoal.hugpkg")
        .map_err(|_| InstallTestError::Effect)?;
    let installed = file
        .inspect(PACKAGE_LIMIT)
        .map_err(|_| InstallTestError::Effect)?;
    let repeated = file
        .inspect(PACKAGE_LIMIT)
        .map_err(|_| InstallTestError::Effect)?;
    (installed.as_deref() == Some(package.archive()) && installed == repeated)
        .then_some(())
        .ok_or(InstallTestError::Effect)
}

pub(super) fn observe_cache(
    root: ConfinedRoot,
    plan: &PackagePlan,
    package: &PackageSnapshot,
) -> Result<(), InstallTestError> {
    let expectation = CacheExpectation::new(
        package.context_id().into(),
        package.candidate_id().into(),
        root.root_id().into(),
        "local-harness-plugins".into(),
        "harness-ultragoal".into(),
        plan.version().into(),
        package.identity().tree_sha256().into(),
    )
    .map_err(|_| InstallTestError::Effect)?;
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema":"harness-ultragoal.codex-cache-observation.v1",
        "context_id":package.context_id(), "candidate_id":package.candidate_id(),
        "cache_root_id":root.root_id(), "entries":[{"marketplace":"local-harness-plugins",
        "plugin_id":"harness-ultragoal", "version":plan.version(),
        "package_tree_sha256":package.identity().tree_sha256()}]
    }))
    .map_err(|_| InstallTestError::Effect)?;
    let mut cache =
        ScopedFile::new(root, "cache/observation.json").map_err(|_| InstallTestError::Effect)?;
    cache
        .apply(None, Some(&bytes))
        .map_err(|_| InstallTestError::Effect)?
        .then_some(())
        .ok_or(InstallTestError::Effect)?;
    reconcile_cache_file(&mut cache, &expectation).map_err(|_| InstallTestError::Effect)?;
    Ok(())
}
