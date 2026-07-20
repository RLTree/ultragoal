mod execution;
#[cfg(test)]
mod host_custody;
mod model;
mod plan;

pub use execution::{apply, recover, recovery_token, verify};
#[cfg(test)]
pub(crate) use host_custody::{
    HostEffectExecutionBinding, HostLifecycleCustody, HostLifecycleObservedBundle,
    HostLifecycleRecord,
};
#[cfg(test)]
pub(crate) use host_custody::{HostLifecycleBinding, HostLifecycleExpectedObservations};
pub use model::{
    ApplyDisposition, ApplyReport, LifecycleAuthorization, LifecycleEffect, LifecycleEffectAdapter,
    LifecycleError, LifecycleIntent, LifecyclePlan, LifecycleRequest, LifecycleState,
    PackageAuthority, RecoveryToken, Version,
};
pub use plan::plan;
