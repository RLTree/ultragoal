//! Public lifecycle data and read-only planning/verification.
//!
//! Mutation authority is crate-private. External callers cannot import the
//! adapter, apply route, recovery route, or recovery-token issuer:
//!
//! ```compile_fail,E0432
//! use ultragoal::plugin_product::lifecycle::apply;
//! ```
//!
//! ```compile_fail,E0603
//! use ultragoal::plugin_product::distribution_adapter::DistributionLifecycleOperation;
//! ```

mod execution;
mod host_custody;
mod model;
mod plan;
#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

pub use execution::verify;
pub(crate) use execution::{apply, recover, recovery_token};
pub(crate) use host_custody::{
    HostEffectExecutionBinding, HostLifecycleCustody, HostLifecycleObservedBundle,
    HostLifecycleRecord,
};
pub(crate) use host_custody::{HostLifecycleBinding, HostLifecycleExpectedObservations};
pub(crate) use model::LifecycleEffectAdapter;
pub use model::{
    ApplyDisposition, ApplyReport, LifecycleAuthorization, LifecycleEffect, LifecycleError,
    LifecycleIntent, LifecyclePlan, LifecycleRequest, LifecycleState, PackageAuthority,
    RecoveryToken, Version,
};
pub use plan::plan;
