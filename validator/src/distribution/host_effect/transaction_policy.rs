use super::HostEffectExecutionPolicy;
use super::lifecycle::{
    AcceptedHostState, AcceptedLifecycleOperation, AcceptedLifecyclePlan,
    AcceptedReconciliationPolicy, AcceptedRollbackPolicy, DescriptorExecutionAdapter,
    DescriptorExecutionCapability, DescriptorExecutionPlatform, DescriptorExecutionPrimitive,
    RootTrustedClock, SupportedHostLifecycleError, SupportedHostLifecycleErrorId,
    TrustedTimeSample, lifecycle_error,
};
use crate::distribution::PackageIdentity;
use crate::plugin_product::lifecycle::{HostLifecycleCustody, LifecycleIntent, LifecycleState};
use std::path::Path;
use std::time::SystemTime;

pub(crate) fn isolated_codex_policy(
    environment: &[(String, String)],
) -> Result<HostEffectExecutionPolicy, &'static str> {
    if environment.len() != 2
        || environment[0].0 != "CODEX_HOME"
        || environment[1].0 != "HOME"
        || environment[0].1 != environment[1].1
    {
        return Err("isolated Codex environment binding invalid");
    }
    let home = Path::new(&environment[0].1);
    let canonical = home
        .canonicalize()
        .map_err(|_| "isolated Codex home unavailable")?;
    if canonical.display().to_string() != environment[0].1 {
        return Err("isolated Codex home binding is not canonical");
    }
    HostEffectExecutionPolicy::strict_isolated_codex_home(30_000, home)
        .map_err(|_| "isolated Codex execution policy failed")
}

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
    let (platform, primitive) = match DescriptorExecutionPlatform::current() {
        DescriptorExecutionPlatform::Darwin => (
            DescriptorExecutionPlatform::Darwin,
            DescriptorExecutionPrimitive::DarwinPosixSpawnSuspendedLoadedVnode,
        ),
        DescriptorExecutionPlatform::Linux => (
            DescriptorExecutionPlatform::Linux,
            DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        ),
        DescriptorExecutionPlatform::FreeBsd => (
            DescriptorExecutionPlatform::FreeBsd,
            DescriptorExecutionPrimitive::Fexecve,
        ),
        DescriptorExecutionPlatform::Other => {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
            ));
        }
    };
    if platform == DescriptorExecutionPlatform::Darwin {
        // Keep the platform probe typed, but do not issue a usable capability:
        // the Darwin launch primitive is intentionally unsupported until a
        // byte-sealing handoff exists.
        let _candidate = DescriptorExecutionCapability::new(
            platform,
            primitive,
            "harness-host-effect".into(),
            "v1".into(),
        )?;
        return Err(lifecycle_error(
            SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
        ));
    }
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
