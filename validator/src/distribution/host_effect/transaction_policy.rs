use super::lifecycle::{
    AcceptedHostState, AcceptedLifecycleOperation, AcceptedLifecyclePlan,
    AcceptedReconciliationPolicy, AcceptedRollbackPolicy, DescriptorExecutionAdapter,
    DescriptorExecutionCapability, DescriptorExecutionPlatform, DescriptorExecutionPrimitive,
    RootTrustedClock, SupportedHostLifecycleError, SupportedHostLifecycleErrorId,
    TrustedTimeSample, lifecycle_error,
};
use crate::distribution::PackageIdentity;
use crate::plugin_product::lifecycle::{HostLifecycleCustody, LifecycleIntent, LifecycleState};
use std::time::SystemTime;

pub(crate) fn accepted_lifecycle(
    custody: &HostLifecycleCustody,
    package: &PackageIdentity,
) -> Result<AcceptedLifecyclePlan, &'static str> {
    let state = |state: &LifecycleState| {
        AcceptedHostState::new(
            state.generation,
            state.installed.as_ref().map(|_| package.clone()),
            state.recovery_required,
        )
        .map_err(|_| "accepted lifecycle state unavailable")
    };
    let before = state(custody.before())?;
    let after = state(custody.expected_after())?;
    let rollback = state(custody.before())?;
    let (operation, rollback_policy, reconciliation_policy) = policies(custody.intent());
    AcceptedLifecyclePlan::new(
        operation,
        before,
        after,
        rollback,
        rollback_policy,
        reconciliation_policy,
    )
    .map_err(|_| "accepted lifecycle plan unavailable")
}

fn policies(
    intent: LifecycleIntent,
) -> (
    AcceptedLifecycleOperation,
    AcceptedRollbackPolicy,
    AcceptedReconciliationPolicy,
) {
    use AcceptedLifecycleOperation as O;
    use AcceptedReconciliationPolicy as R;
    use AcceptedRollbackPolicy as B;
    match intent {
        LifecycleIntent::FreshInstall => (
            O::FreshInstall,
            B::RemoveOnlyNewTarget,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::MonotonicUpdate => (
            O::MonotonicUpdate,
            B::RestoreExactPreState,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::FailedUpdateRecovery => (
            O::FailedUpdateRecovery,
            B::RestoreExactPreState,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::AuthorizedRollback => (
            O::AuthorizedRollback,
            B::RestoreExactPreState,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::IdempotentReinstall => (
            O::IdempotentReinstall,
            B::ManualReconciliationOnly,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::UninstallTeardown => (
            O::UninstallTeardown,
            B::RestoreExactPreState,
            R::ExactAbsenceAndSeparateHostLayers,
        ),
        LifecycleIntent::StaleCacheRecovery => (
            O::StaleCacheRecovery,
            B::RestoreExactPreState,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::RepeatUse => (
            O::RepeatUse,
            B::ManualReconciliationOnly,
            R::ExactPostStateAndSeparateHostLayers,
        ),
    }
}

pub(crate) struct CurrentDescriptorAdapter;

impl DescriptorExecutionAdapter for CurrentDescriptorAdapter {
    fn descriptor_capability(
        &mut self,
    ) -> Result<DescriptorExecutionCapability, SupportedHostLifecycleError> {
        current_capability()
    }
}

pub(crate) fn current_capability()
-> Result<DescriptorExecutionCapability, SupportedHostLifecycleError> {
    let (platform, primitive) = if cfg!(target_os = "macos") {
        (
            DescriptorExecutionPlatform::Darwin,
            DescriptorExecutionPrimitive::DarwinPosixSpawnSuspendedLoadedVnode,
        )
    } else if cfg!(target_os = "linux") {
        (
            DescriptorExecutionPlatform::Linux,
            DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        )
    } else {
        (
            DescriptorExecutionPlatform::FreeBsd,
            DescriptorExecutionPrimitive::Fexecve,
        )
    };
    DescriptorExecutionCapability::new(
        platform,
        primitive,
        "harness-host-effect".into(),
        "v1".into(),
    )
}

#[derive(Default)]
pub(crate) struct SystemTrustedClock {
    sequence: u64,
}

impl RootTrustedClock for SystemTrustedClock {
    fn sample(&mut self) -> Result<TrustedTimeSample, SupportedHostLifecycleError> {
        self.sequence = self.sequence.saturating_add(1);
        let unix_ms = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::UntrustedTime))?
            .as_millis() as u64;
        TrustedTimeSample::new("system-clock".into(), 1, self.sequence, unix_ms)
    }
}
