const SESSION_NONCE_BYTES: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AcceptedLifecycleOperation {
    FreshInstall,
    MonotonicUpdate,
    FailedUpdateRecovery,
    AuthorizedRollback,
    IdempotentReinstall,
    UninstallTeardown,
    StaleCacheRecovery,
    RepeatUse,
}

impl AcceptedLifecycleOperation {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::FreshInstall => "fresh-install",
            Self::MonotonicUpdate => "monotonic-update",
            Self::FailedUpdateRecovery => "failed-update-recovery",
            Self::AuthorizedRollback => "authorized-rollback",
            Self::IdempotentReinstall => "idempotent-reinstall",
            Self::UninstallTeardown => "uninstall-teardown",
            Self::StaleCacheRecovery => "stale-cache-recovery",
            Self::RepeatUse => "repeat-use",
        }
    }

    fn is_effectful(self) -> bool {
        !matches!(self, Self::IdempotentReinstall | Self::RepeatUse)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AcceptedHostState {
    generation: u64,
    package: Option<PackageIdentity>,
    recovery_required: bool,
}

impl AcceptedHostState {
    pub(in crate::distribution::host_effect) fn new(
        generation: u64,
        package: Option<PackageIdentity>,
        recovery_required: bool,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if let Some(package) = &package {
            package.validate().map_err(|_| invalid())?;
        }
        Ok(Self {
            generation,
            package,
            recovery_required,
        })
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AcceptedRollbackPolicy {
    RestoreExactPreState,
    RemoveOnlyNewTarget,
    ManualReconciliationOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AcceptedReconciliationPolicy {
    ExactPostStateAndSeparateHostLayers,
    ExactAbsenceAndSeparateHostLayers,
    AmbiguousOutcomeRequiresManualReview,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AcceptedLifecyclePlan {
    operation: AcceptedLifecycleOperation,
    before: AcceptedHostState,
    expected_after: AcceptedHostState,
    rollback_state: AcceptedHostState,
    rollback_policy: AcceptedRollbackPolicy,
    reconciliation_policy: AcceptedReconciliationPolicy,
    plan_sha256: String,
}

impl AcceptedLifecyclePlan {
    pub(in crate::distribution::host_effect) fn new(
        operation: AcceptedLifecycleOperation,
        before: AcceptedHostState,
        expected_after: AcceptedHostState,
        rollback_state: AcceptedHostState,
        rollback_policy: AcceptedRollbackPolicy,
        reconciliation_policy: AcceptedReconciliationPolicy,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !matches_lifecycle_operation(
            operation,
            &before,
            &expected_after,
            &rollback_state,
            rollback_policy,
            reconciliation_policy,
        ) {
            return Err(invalid());
        }
        #[derive(Serialize)]
        struct Plan<'a> {
            schema: &'static str,
            operation: AcceptedLifecycleOperation,
            before: &'a AcceptedHostState,
            expected_after: &'a AcceptedHostState,
            rollback_state: &'a AcceptedHostState,
            rollback_policy: AcceptedRollbackPolicy,
            reconciliation_policy: AcceptedReconciliationPolicy,
        }
        let plan_sha256 = digest_json(&Plan {
            schema: "harness-ultragoal.accepted-host-lifecycle-plan.v1",
            operation,
            before: &before,
            expected_after: &expected_after,
            rollback_state: &rollback_state,
            rollback_policy,
            reconciliation_policy,
        })?;
        Ok(Self {
            operation,
            before,
            expected_after,
            rollback_state,
            rollback_policy,
            reconciliation_policy,
            plan_sha256,
        })
    }

    pub(crate) const fn operation(&self) -> AcceptedLifecycleOperation {
        self.operation
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

fn matches_lifecycle_operation(
    operation: AcceptedLifecycleOperation,
    before: &AcceptedHostState,
    expected_after: &AcceptedHostState,
    rollback_state: &AcceptedHostState,
    rollback_policy: AcceptedRollbackPolicy,
    reconciliation_policy: AcceptedReconciliationPolicy,
) -> bool {
    use AcceptedLifecycleOperation::*;
    use AcceptedReconciliationPolicy::*;
    use AcceptedRollbackPolicy::*;

    let increments_once = before.generation().checked_add(1) == Some(expected_after.generation());
    let restores_before = rollback_state == before;
    let before_has_package = before.package.is_some();
    let after_has_package = expected_after.package.is_some();
    let package_changed = before.package != expected_after.package;
    let recovery_cleared = before.recovery_required && !expected_after.recovery_required;
    match operation {
        FreshInstall => {
            !before_has_package
                && !before.recovery_required
                && after_has_package
                && !expected_after.recovery_required
                && increments_once
                && restores_before
                && rollback_policy == RemoveOnlyNewTarget
                && reconciliation_policy == ExactPostStateAndSeparateHostLayers
        }
        MonotonicUpdate => {
            before_has_package
                && !before.recovery_required
                && after_has_package
                && !expected_after.recovery_required
                && package_changed
                && increments_once
                && restores_before
                && rollback_policy == RestoreExactPreState
                && reconciliation_policy == ExactPostStateAndSeparateHostLayers
        }
        FailedUpdateRecovery => {
            before_has_package
                && recovery_cleared
                && before.package == expected_after.package
                && increments_once
                && restores_before
                && rollback_policy == ManualReconciliationOnly
                && reconciliation_policy == ExactPostStateAndSeparateHostLayers
        }
        AuthorizedRollback => {
            before_has_package
                && !before.recovery_required
                && after_has_package
                && !expected_after.recovery_required
                && package_changed
                && increments_once
                && restores_before
                && rollback_policy == RestoreExactPreState
                && reconciliation_policy == ExactPostStateAndSeparateHostLayers
        }
        IdempotentReinstall | RepeatUse => {
            before_has_package
                && !before.recovery_required
                && expected_after == before
                && restores_before
                && rollback_policy == RestoreExactPreState
                && reconciliation_policy == ExactPostStateAndSeparateHostLayers
        }
        UninstallTeardown => {
            before_has_package
                && !before.recovery_required
                && !after_has_package
                && !expected_after.recovery_required
                && increments_once
                && restores_before
                && rollback_policy == RemoveOnlyNewTarget
                && reconciliation_policy == ExactAbsenceAndSeparateHostLayers
        }
        // A one-package host state cannot distinguish stale cache from a
        // current package. Do not turn that ambiguity into host authority.
        StaleCacheRecovery => false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum AcceptedHostScope {
    Personal {
        home_id: String,
        host_id: String,
        marketplace: String,
    },
    Repository {
        home_id: String,
        project_id: String,
        host_id: String,
        marketplace: String,
    },
}
