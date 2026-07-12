mod cache;
mod cache_observation;
mod error;
mod filesystem;
mod host;
mod host_capability;
mod install;
mod json;
mod marketplace;
mod marketplace_observation;
mod model;
mod observations;
mod package;
mod reader;
mod registry_observation;
mod runtime_probe;
mod spec;
mod supply;
mod verify;

pub use cache::{CacheExpectation, CacheSnapshot, reconcile_cache_read_only};
pub use cache_observation::{CacheReader, reconcile_cache_file};
pub use error::{DistributionError, DistributionErrorId};
pub use filesystem::{ConfinedRoot, ScopedFile, ScopedInstall, ScopedTree};
#[cfg(all(test, unix))]
pub(crate) use filesystem::{
    EffectPoint, assert_test_effect_hook_consumed, set_test_effect_hook,
    set_test_effect_hook_matching,
};
pub use host::{
    CommandOutput, HostAuthorization, HostCommand, HostCommandPlan, HostExecutionSnapshot,
    HostExecutor, execute_authorized,
};
pub use host_capability::{
    HostAdapterKind, HostCapabilityDeclaration, HostCapabilityState, JourneyBinding,
};
pub use install::{
    ExpectedPrior, InstallEffects, InstallPlan, InstallScope, InstallSnapshot, InstallTransaction,
    install, rollback_install, uninstall,
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
    reject_stale_version_reuse, verify_bound_surface_chain, verify_surface_chain,
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
pub use registry_observation::{
    AppRegistryObservation, AppRegistryVerdict, DiscoveryObservation, DiscoveryVerdict,
    RegistryObservations, RegistryReader, observe_app_registry, observe_discovery,
    observe_registry_file, registry_document,
};
pub use runtime_probe::{RuntimeProbePlan, execute_runtime_probe};
pub use supply::{
    ProvenanceExpectation, ProvenanceSnapshot, SignatureExpectation, SignatureSnapshot,
    SignatureVerifierEffects, verify_provenance, verify_signature,
};
pub use verify::{verify, verify_identity_ladder};
