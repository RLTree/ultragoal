use super::isolated_root::IsolatedRoot;
use super::observation::{materialize, observe_cache, observe_installed, package_authority};
use crate::context::LiveContext;
use crate::distribution::{
    ConfinedRoot, ExpectedPrior, InstallPlan, InstallScope, ScopedFile, ScopedInstall,
    capture_product_package, install, plan_package_from_inventory, verify_package,
    verify_product_package,
};
use crate::inventory::{AuthorityCatalog, InventoryBuilder};
use crate::plugin_product::lifecycle::{
    HostLifecycleCustody, LifecycleAuthorization, LifecycleIntent, LifecycleRequest,
    LifecycleState, plan,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const PACKAGE_LIMIT: usize = 65 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InstallTestError {
    Context,
    Input,
    Package,
    Custody,
    Effect,
    Output,
}

#[derive(Serialize)]
pub(crate) struct InstallTestReport {
    schema_version: &'static str,
    candidate_id: String,
    package_sha256: String,
    output: String,
    materialized_tree_sha256: String,
    installed_postimage: &'static str,
    cache_postimage: &'static str,
    marketplace: &'static str,
    app_registry: &'static str,
    discovery: &'static str,
    runtime: &'static str,
    claim_ceiling: &'static str,
}

pub(crate) fn execute(
    context: &LiveContext,
    input_path: &str,
    output_path: &str,
) -> Result<InstallTestReport, InstallTestError> {
    let catalog = InventoryBuilder::new(context)
        .build()
        .map_err(|_| InstallTestError::Context)?;
    let artifact =
        capture_product_package(context, &catalog).map_err(|_| InstallTestError::Package)?;
    verify_product_package(&artifact, context, &catalog).map_err(|_| InstallTestError::Package)?;
    let workspace = ConfinedRoot::open_workspace(context).map_err(|_| InstallTestError::Context)?;
    let input =
        ScopedFile::new(workspace.clone(), input_path).map_err(|_| InstallTestError::Input)?;
    let input_bytes = input
        .inspect(PACKAGE_LIMIT)
        .map_err(|_| InstallTestError::Input)?
        .ok_or(InstallTestError::Input)?;
    if input_bytes != artifact.snapshot().archive() {
        return Err(InstallTestError::Package);
    }
    let package_plan =
        plan_package_from_inventory(context.worktree_root(), artifact.source_inventory())
            .map_err(|_| InstallTestError::Package)?;
    let package =
        verify_package(&package_plan, &input_bytes).map_err(|_| InstallTestError::Package)?;
    let isolated = IsolatedRoot::create().map_err(|_| InstallTestError::Custody)?;
    let mut report = install_current_package(
        context,
        &catalog,
        &artifact,
        &package_plan,
        &package,
        isolated,
    )?;
    report.output = output_path.to_owned();
    let encoded = serde_json::to_vec(&report).map_err(|_| InstallTestError::Output)?;
    if encoded.len() > 16 * 1024 * 1024 {
        return Err(InstallTestError::Output);
    }
    let output = ScopedFile::new(workspace, output_path).map_err(|_| InstallTestError::Output)?;
    output
        .apply(None, Some(&encoded))
        .map_err(|_| InstallTestError::Output)?
        .then_some(())
        .ok_or(InstallTestError::Output)?;
    Ok(report)
}

fn install_current_package(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    artifact: &crate::distribution::ProductionPackageArtifact,
    package_plan: &crate::distribution::PackagePlan,
    package: &crate::distribution::PackageSnapshot,
    isolated: IsolatedRoot,
) -> Result<InstallTestReport, InstallTestError> {
    let authority = package_authority(package_plan, package)?;
    let mut custody = HostLifecycleCustody::take(
        plan(
            &LifecycleState::default(),
            &LifecycleRequest {
                intent: LifecycleIntent::FreshInstall,
                target: Some(authority),
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: true,
                    allow_downgrade: false,
                    expected_installed_sha256: None,
                },
            },
        )
        .map_err(|_| InstallTestError::Custody)?,
    )
    .map_err(|_| InstallTestError::Custody)?;
    let admission = crate::distribution::host_effect::admit_isolated_lifecycle(
        isolated.path(),
        custody.pre_effect_record().clone(),
        context.context_id(),
        package.candidate_id(),
        package.package_sha256(),
        &digest(isolated.confined().root_id().as_bytes()),
    )
    .map_err(|_| InstallTestError::Custody)?;
    let binding = admission
        .begin_effects(&mut custody)
        .map_err(|_| InstallTestError::Custody)?;
    let materialized_tree_sha256 = materialize(package_plan, isolated.confined())?;
    let mut installed = ScopedInstall::new(isolated.confined());
    let install_plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        InstallScope::PersonalFixture,
        "plugins/harness-ultragoal.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .map_err(|_| InstallTestError::Effect)?;
    install(&install_plan, package, &mut installed).map_err(|_| InstallTestError::Effect)?;
    observe_installed(isolated.confined(), package)?;
    observe_cache(isolated.confined(), package_plan, package)?;
    verify_product_package(artifact, context, catalog).map_err(|_| InstallTestError::Package)?;
    let completed = custody.effects().to_vec();
    admission
        .settle(
            &mut custody,
            binding,
            custody.expected_after().clone(),
            completed,
        )
        .map_err(|_| InstallTestError::Custody)?;
    let report = InstallTestReport {
        schema_version: "HarnessIsolatedInstallTest-v1",
        candidate_id: package.candidate_id().into(),
        package_sha256: package.package_sha256().into(),
        output: String::new(),
        materialized_tree_sha256,
        installed_postimage: "verified",
        cache_postimage: "verified",
        marketplace: "unavailable",
        app_registry: "unavailable",
        discovery: "unavailable",
        runtime: "unavailable",
        claim_ceiling: "isolated package, install, and cache observations verified; marketplace, app-registry, discovery, runtime, and real-host claims unavailable",
    };
    isolated.remove().map_err(|_| InstallTestError::Custody)?;
    Ok(report)
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
