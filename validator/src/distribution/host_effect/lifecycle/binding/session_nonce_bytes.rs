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
        if before.generation() > expected_after.generation()
            || rollback_state.generation() < before.generation()
            || (operation == AcceptedLifecycleOperation::UninstallTeardown
                && expected_after.package.is_some())
            || (operation == AcceptedLifecycleOperation::FreshInstall
                && expected_after.package.is_none())
        {
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
