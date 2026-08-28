//! Generic lifecycle data and read-only planning/verification.
//!
//! This model is a non-personal verifier fixture. It cannot authorize Codex
//! plugin installation, update, removal, rollback, restoration, or recovery.
//! Mutation adapters are crate-private and unreachable from public package
//! routing. External callers cannot import the adapter, apply route, recovery
//! route, or recovery-token issuer:
//!
//! ```compile_fail,E0432
//! use ultragoal::plugin_product::lifecycle::apply;
//! ```
//!
//! ```compile_fail,E0603
//! use ultragoal::plugin_product::distribution_adapter::DistributionLifecycleOperation;
//! ```

pub(crate) mod execution;
mod host_custody;
pub(crate) mod model;
mod plan;
#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
mod verification;

#[cfg(test)]
fn read(path: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repository parent");
    std::fs::read_to_string(root.join(path)).expect("lifecycle fixture readable")
}

#[cfg(test)]
pub(crate) use execution::{apply, recover, recovery_token};
pub(crate) use host_custody::HostLifecycleRecoveryDisposition;
pub(crate) use host_custody::recovery_state_after_completed_prefix;
pub(crate) use host_custody::{
    HostCommandObservation, HostEffectExecutionBinding, HostLifecycleCustody,
    HostLifecycleFinalization, HostLifecycleObservedBundle, HostLifecycleRecord,
};
pub(crate) use host_custody::{HostLifecycleBinding, HostLifecycleExpectedObservations};
#[cfg(test)]
pub(crate) use model::LifecycleEffectAdapter;
pub(crate) use model::PriorInstalledAuthority;
#[cfg(test)]
pub(crate) use model::RecoveryToken;
pub use model::{
    ApplyDisposition, ApplyReport, LifecycleAuthorization, LifecycleEffect, LifecycleError,
    LifecycleIntent, LifecyclePlan, LifecycleRequest, LifecycleState, PackageAuthority, Version,
};
pub use plan::plan;
pub use verification::verify;
