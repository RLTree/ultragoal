mod cache;
mod cache_observation;
mod error;
mod filesystem;
mod host;
mod host_capability;
pub(crate) mod host_effect;
mod install;
mod json;
mod marketplace;
mod marketplace_observation;
mod model;
mod observations;
mod package;
mod reader;
#[cfg(test)]
pub(crate) mod registry_observation;
#[cfg(not(test))]
mod registry_observation;
mod runtime_probe;
mod spec;
mod supply;
mod verify;

/// A bounded adapter failure. Callers retain their domain-specific failure mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectFailure;

pub use cache::{CacheExpectation, CacheSnapshot, reconcile_cache_read_only};
pub use cache_observation::{CacheReader, publish_cache_file, reconcile_cache_file};
pub use error::{DistributionError, DistributionErrorId};
pub(crate) use filesystem::ReadOnlyWorkspace;
pub use filesystem::{ConfinedRoot, ScopedFile, ScopedInstall, ScopedTree};
#[cfg(all(test, unix))]
pub(crate) use filesystem::{
    EffectPoint, assert_test_effect_hook_consumed, set_test_effect_hook_matching,
};
pub use host::{HostCommand, HostCommandPlan};
pub use host_capability::{
    HostAdapterKind, HostCapabilityDeclaration, HostCapabilityState, JourneyBinding,
};
pub(crate) use host_effect::resolve_codex_executable;
pub use install::{
    ExpectedPrior, InstallEffects, InstallPlan, InstallScope, InstallSnapshot, InstallTransaction,
    RollbackInstallError, install, rollback_install, uninstall,
};
pub use marketplace::{
    CodexMarketplace, CodexPlugin, MarketplaceEffects, MarketplaceExpectation, MarketplacePlan,
    MarketplaceScope, MarketplaceSnapshot, MarketplaceTransaction, MarketplaceVerdict,
    apply_marketplace, plan_codex_marketplace, rollback_marketplace, unavailable_marketplace,
    verify_marketplace,
};
pub use marketplace_observation::observe_codex_marketplace;
pub use model::{
    Capability, DistributionReport, HostVerdict, IdentitySurface, JoinReport, JoinVerdict, Layer,
    LayerReport, LayerVerdict, PackageIdentity, SourceIdentity, SurfaceIdentity,
    reject_stale_version_reuse, verify_bound_surface_chain,
};
pub use observations::{RuntimeObservation, RuntimeVerdict};
pub use package::{
    ExpectedTree, MaterializeEffects, MaterializeTransaction, PackageArtifactBinding,
    PackageArtifactTransaction, PackageEffects, PackageEntry, PackagePlan, PackageRole,
    PackageSnapshot, TreeObject, TreeObjectKind, build_package, materialize_package, plan_package,
    plan_package_from_inventory, publish_package_artifact, reconcile_materialized_tree,
    reconcile_package_artifact, recover_package_artifact, rollback_materialization,
    rollback_package_artifact, tree_sha256, verify_package,
};
#[cfg(not(test))]
pub use package::{
    MarketplaceSourceObservation, ProductionPackageArtifact, ProductionPackageError,
    ProductionPackageErrorId, ProductionPackageSession, capture_product_package,
    verify_product_package,
};
#[cfg(test)]
pub(crate) use package::{
    ProductionPackageArtifact, ProductionPackageErrorId, capture_product_package,
    verify_product_package,
};
pub use registry_observation::{
    AppRegistryObservation, AppRegistryVerdict, DiscoveryObservation, DiscoveryVerdict,
    RegistryObservations, observe_app_registry, observe_discovery, observe_discovery_file,
    observe_registry_file, observe_supported_host_discovery, publish_discovery_file,
    publish_registry_file, registry_document,
};
pub use runtime_probe::{
    InstalledPackageRuntimeProbeRequest, RuntimeProbePlan, execute_runtime_probe,
    publish_installed_runtime_probe,
};
pub use supply::{
    ProvenanceExpectation, ProvenanceSnapshot, SignatureExpectation, SignatureSnapshot,
    SignatureVerifierEffects, verify_provenance, verify_signature,
};
pub use verify::{verify, verify_identity_ladder};
