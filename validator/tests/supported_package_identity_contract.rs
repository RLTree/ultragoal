use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use ultragoal::distribution::{
    CacheExpectation, CodexPlugin, DistributionErrorId, ExpectedPrior, HostCapabilityDeclaration,
    InstallEffects, InstallPlan, InstallScope, JourneyBinding, MarketplaceScope, PackageEffects,
    PackageIdentity, PackagePlan, RuntimeProbePlan, SurfaceIdentity, build_package, install,
    observe_app_registry, observe_codex_marketplace, observe_discovery, plan_codex_marketplace,
    reconcile_cache_read_only, registry_document, unavailable_marketplace,
    verify_bound_surface_chain, verify_marketplace, verify_package,
};
use ultragoal::orchestration::{
    Actor, ArtifactWorkspace, Binding, CanonicalPath, EffectClass, EffectGrant, LeaseSpec,
    OwnedScope, PrerequisiteEvidence, Principal, SafetyClass, ScopePolicy, WorkPackage,
    WorkerResultV1,
};

include!("supported_package_identity_contract/context.rs");

include!("supported_package_identity_contract/layout.rs");

include!(
    "supported_package_identity_contract/typed_identity_surfaces_bind_only_verified_same_candidate_observations.rs"
);

include!("supported_package_identity_contract/cache_identity/mod.rs");

include!(
    "supported_package_identity_contract/independent_decoder_reinventories_without_writes_and_rejects_every_structural_substitution.rs"
);

include!("supported_package_identity_contract/corrective_lease.rs");
