use super::{SupportedHostLifecycleError, SupportedHostLifecycleErrorId, lifecycle_error};
use crate::distribution::{
    Capability, HostCapabilityDeclaration, HostCapabilityState, HostCommandPlan, JourneyBinding,
    PackageIdentity,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::Metadata;
use std::path::Path;

use super::super::{
    HostEffectDecision, HostEffectLedgerHead, HostEffectPermitBinding, PinnedHostExecutable,
};

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

impl AcceptedHostScope {
    pub(in crate::distribution::host_effect) fn personal(
        journey: &JourneyBinding,
        marketplace: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        validate_name(&marketplace)?;
        Ok(Self::Personal {
            home_id: journey.home_id().to_owned(),
            host_id: journey.host_id().to_owned(),
            marketplace,
        })
    }

    pub(in crate::distribution::host_effect) fn repository(
        journey: &JourneyBinding,
        marketplace: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        validate_name(&marketplace)?;
        Ok(Self::Repository {
            home_id: journey.home_id().to_owned(),
            project_id: journey.project_id().to_owned(),
            host_id: journey.host_id().to_owned(),
            marketplace,
        })
    }

    fn validate_for(&self, journey: &JourneyBinding) -> Result<(), SupportedHostLifecycleError> {
        let (home_id, project_id, host_id, marketplace) = match self {
            Self::Personal {
                home_id,
                host_id,
                marketplace,
            } => (home_id, None, host_id, marketplace),
            Self::Repository {
                home_id,
                project_id,
                host_id,
                marketplace,
            } => (home_id, Some(project_id), host_id, marketplace),
        };
        validate_name(marketplace)?;
        if home_id != journey.home_id()
            || host_id != journey.host_id()
            || project_id.is_some_and(|value| value != journey.project_id())
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn required_capabilities(&self, operation: AcceptedLifecycleOperation) -> Vec<Capability> {
        if !operation.is_effectful() {
            return Vec::new();
        }
        let mut required = vec![Capability::Install, Capability::Marketplace];
        if matches!(self, Self::Repository { .. }) {
            required.insert(0, Capability::Filesystem);
        }
        required
    }

    fn validate_plan(
        &self,
        package: &PackageIdentity,
        operation: AcceptedLifecycleOperation,
        plan: &HostCommandPlan,
    ) -> Result<(), SupportedHostLifecycleError> {
        let remove = operation == AcceptedLifecycleOperation::UninstallTeardown;
        let expected = match self {
            Self::Personal { marketplace, .. } if remove => {
                HostCommandPlan::personal_remove(package, marketplace)
            }
            Self::Personal { marketplace, .. } => {
                HostCommandPlan::personal_install(package, marketplace)
            }
            Self::Repository { marketplace, .. } if remove => {
                HostCommandPlan::repository_remove(package, marketplace)
            }
            Self::Repository {
                project_id,
                marketplace,
                ..
            } => {
                let repository_root = plan
                    .commands()
                    .first()
                    .filter(|command| {
                        command.program() == "codex"
                            && command.argv().len() == 4
                            && command.argv()[..3] == ["plugin", "marketplace", "add"]
                    })
                    .map(|command| command.argv()[3].as_str())
                    .ok_or_else(invalid)?;
                if project_identity(repository_root)? != *project_id {
                    return Err(invalid());
                }
                HostCommandPlan::repository_install(package, repository_root, marketplace)
            }
        }
        .map_err(|_| invalid())?;
        if expected.plan_sha256() != plan.plan_sha256() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::PlanSubstitution,
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostObjectIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    hard_links: u64,
    byte_length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl HostObjectIdentity {
    #[cfg(unix)]
    pub(in crate::distribution::host_effect) fn from_metadata(
        metadata: &Metadata,
    ) -> Result<Self, SupportedHostLifecycleError> {
        use std::os::unix::fs::MetadataExt;
        if !metadata.is_dir() && !metadata.is_file() {
            return Err(invalid());
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            hard_links: metadata.nlink(),
            byte_length: metadata.size(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    #[cfg(not(unix))]
    pub(in crate::distribution::host_effect) fn from_metadata(
        _metadata: &Metadata,
    ) -> Result<Self, SupportedHostLifecycleError> {
        Err(invalid())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ObservedTargetIdentity {
    scope_sha256: String,
    generation: u64,
    object: HostObjectIdentity,
    target_sha256: String,
}

impl ObservedTargetIdentity {
    pub(in crate::distribution::host_effect) fn new(
        scope: &AcceptedHostScope,
        generation: u64,
        object: HostObjectIdentity,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if generation == 0 {
            return Err(invalid());
        }
        let scope_sha256 = digest_json(&ScopeBinding {
            schema: "harness-ultragoal.accepted-host-scope.v1",
            scope,
        })?;
        #[derive(Serialize)]
        struct Target<'a> {
            schema: &'static str,
            scope_sha256: &'a str,
            generation: u64,
            object: &'a HostObjectIdentity,
        }
        let target_sha256 = digest_json(&Target {
            schema: "harness-ultragoal.observed-host-target.v1",
            scope_sha256: &scope_sha256,
            generation,
            object: &object,
        })?;
        Ok(Self {
            scope_sha256,
            generation,
            object,
            target_sha256,
        })
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn target_sha256(&self) -> &str {
        &self.target_sha256
    }
}

#[derive(Serialize)]
struct ScopeBinding<'a> {
    schema: &'static str,
    scope: &'a AcceptedHostScope,
}

pub(crate) struct AcceptedHostEffect {
    package: PackageIdentity,
    lifecycle: AcceptedLifecyclePlan,
    scope: AcceptedHostScope,
    expected_target: ObservedTargetIdentity,
    expected_head: HostEffectLedgerHead,
    coordinator_binding_sha256: String,
    package_identity_sha256: String,
    journey_binding_sha256: String,
    session_issuance_sha256: String,
    expected_pre_state_sha256: String,
    expected_post_state_sha256: String,
    rollback_policy_sha256: String,
    reconciliation_policy_sha256: String,
    host_scope_sha256: String,
    host_capability_sha256: String,
    required_capabilities_sha256: String,
    external_request_sha256: String,
    command_plan_sha256: String,
    argv_sha256: String,
    executable_identity_sha256: String,
}

impl std::fmt::Debug for AcceptedHostEffect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AcceptedHostEffect")
            .field("lifecycle_operation", &self.lifecycle.operation())
            .field("target_generation", &self.expected_target.generation())
            .finish_non_exhaustive()
    }
}

impl AcceptedHostEffect {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn accept(
        coordinator_binding_sha256: String,
        package: PackageIdentity,
        journey: JourneyBinding,
        host: HostCapabilityDeclaration,
        lifecycle: AcceptedLifecyclePlan,
        scope: AcceptedHostScope,
        plan: &HostCommandPlan,
        executable: &PinnedHostExecutable,
        expected_target: ObservedTargetIdentity,
        expected_head: HostEffectLedgerHead,
    ) -> Result<Self, SupportedHostLifecycleError> {
        package.validate().map_err(|_| invalid())?;
        if !lifecycle.operation().is_effectful() {
            // The accepted plugin lifecycle emits no external request for
            // repeat use or idempotent reinstall. This effect boundary must
            // not turn either no-op intent into executable authority.
            return Err(invalid());
        }
        if journey.package() != &package
            || journey.capability_sha256() != host.capability_sha256()
            || journey.binding_sha256().len() != 71
        {
            return Err(invalid());
        }
        scope.validate_for(&journey)?;
        scope.validate_plan(&package, lifecycle.operation(), plan)?;
        let required = scope.required_capabilities(lifecycle.operation());
        if required
            .iter()
            .any(|capability| host.state(*capability) != HostCapabilityState::Supported)
        {
            return Err(invalid());
        }
        let host_scope_sha256 = digest_json(&ScopeBinding {
            schema: "harness-ultragoal.accepted-host-scope.v1",
            scope: &scope,
        })?;
        if expected_target.scope_sha256 != host_scope_sha256 {
            return Err(invalid());
        }
        let package_identity_sha256 = digest_json(&PackageBinding {
            schema: "harness-ultragoal.accepted-package-identity.v1",
            package: &package,
        })?;
        let command_plan_sha256 = command_plan_sha256(&package, plan)?;
        if command_plan_sha256 != plan.plan_sha256() {
            return Err(invalid());
        }
        let argv_sha256 = argv_sha256(plan)?;
        let executable_identity_sha256 = executable
            .identity()
            .binding_sha256()
            .map_err(|_| invalid())?;
        let expected_pre_state_sha256 = digest_json(&StateBinding {
            schema: "harness-ultragoal.accepted-pre-state.v1",
            state: &lifecycle.before,
        })?;
        let expected_post_state_sha256 = digest_json(&StateBinding {
            schema: "harness-ultragoal.accepted-post-state.v1",
            state: &lifecycle.expected_after,
        })?;
        let rollback_policy_sha256 = digest_json(&RollbackBinding {
            schema: "harness-ultragoal.accepted-rollback-policy.v1",
            rollback_state: &lifecycle.rollback_state,
            policy: lifecycle.rollback_policy,
        })?;
        let reconciliation_policy_sha256 = digest_json(&ReconciliationBinding {
            schema: "harness-ultragoal.accepted-reconciliation-policy.v1",
            expected_after: &lifecycle.expected_after,
            policy: lifecycle.reconciliation_policy,
        })?;
        let required_capabilities_sha256 = digest_json(&RequiredCapabilityBinding {
            schema: "harness-ultragoal.required-host-capabilities.v1",
            required: &required,
        })?;
        let session_issuance_sha256 = session_issuance(
            &coordinator_binding_sha256,
            &package_identity_sha256,
            journey.binding_sha256(),
            lifecycle.plan_sha256(),
            &host_scope_sha256,
        )?;
        let external_request_sha256 = digest_json(&ExternalRequestBinding {
            schema: "harness-ultragoal.accepted-external-host-request.v1",
            coordinator_binding_sha256: &coordinator_binding_sha256,
            session_issuance_sha256: &session_issuance_sha256,
            lifecycle_plan_sha256: lifecycle.plan_sha256(),
            host_scope_sha256: &host_scope_sha256,
            command_plan_sha256: &command_plan_sha256,
            argv_sha256: &argv_sha256,
            target_identity_sha256: expected_target.target_sha256(),
        })?;
        Ok(Self {
            package,
            lifecycle,
            scope,
            expected_target,
            expected_head,
            coordinator_binding_sha256,
            package_identity_sha256,
            journey_binding_sha256: journey.binding_sha256().to_owned(),
            session_issuance_sha256,
            expected_pre_state_sha256,
            expected_post_state_sha256,
            rollback_policy_sha256,
            reconciliation_policy_sha256,
            host_scope_sha256,
            host_capability_sha256: host.capability_sha256().to_owned(),
            required_capabilities_sha256,
            external_request_sha256,
            command_plan_sha256,
            argv_sha256,
            executable_identity_sha256,
        })
    }

    pub(super) fn require_coordinator(
        &self,
        observed: &str,
    ) -> Result<(), SupportedHostLifecycleError> {
        if self.coordinator_binding_sha256 != observed {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::CoordinatorSubstitution,
            ));
        }
        Ok(())
    }

    pub(super) fn require_plan(
        &self,
        custody: &RootPlanCustody,
    ) -> Result<(), SupportedHostLifecycleError> {
        if custody.plan_sha256() != Some(self.command_plan_sha256.as_str()) {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::PlanSubstitution,
            ));
        }
        Ok(())
    }

    pub(super) fn require_executable(
        &self,
        executable: &PinnedHostExecutable,
    ) -> Result<(), SupportedHostLifecycleError> {
        executable
            .revalidate()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::ExecutableSubstitution))?;
        let observed = executable
            .identity()
            .binding_sha256()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::ExecutableSubstitution))?;
        if observed != self.executable_identity_sha256 {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::ExecutableSubstitution,
            ));
        }
        Ok(())
    }

    pub(super) fn require_target(
        &self,
        target: &ObservedTargetIdentity,
    ) -> Result<(), SupportedHostLifecycleError> {
        if &self.expected_target != target {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::TargetSubstitution,
            ));
        }
        Ok(())
    }

    pub(super) fn expected_target(&self) -> &ObservedTargetIdentity {
        &self.expected_target
    }

    pub(super) fn expected_head(&self) -> &HostEffectLedgerHead {
        &self.expected_head
    }

    pub(super) fn derive_binding(
        &self,
        issued_at_unix_ms: u64,
        expires_at_unix_ms: u64,
        current_head: &HostEffectLedgerHead,
    ) -> Result<HostEffectPermitBinding, SupportedHostLifecycleError> {
        if current_head != &self.expected_head || issued_at_unix_ms >= expires_at_unix_ms {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::StaleLedgerHead,
            ));
        }
        Ok(HostEffectPermitBinding {
            context_id: self.package.source().context_id().to_owned(),
            candidate_id: self.package.source().candidate_id().to_owned(),
            package_identity_sha256: self.package_identity_sha256.clone(),
            journey_binding_sha256: self.journey_binding_sha256.clone(),
            session_issuance_sha256: self.session_issuance_sha256.clone(),
            lifecycle_plan_sha256: self.lifecycle.plan_sha256().to_owned(),
            lifecycle_intent: self.lifecycle.operation().as_str().to_owned(),
            expected_pre_state_sha256: self.expected_pre_state_sha256.clone(),
            expected_post_state_sha256: self.expected_post_state_sha256.clone(),
            rollback_policy_sha256: self.rollback_policy_sha256.clone(),
            reconciliation_policy_sha256: self.reconciliation_policy_sha256.clone(),
            host_scope_sha256: self.host_scope_sha256.clone(),
            host_capability_sha256: self.host_capability_sha256.clone(),
            required_capabilities_sha256: self.required_capabilities_sha256.clone(),
            external_request_sha256: self.external_request_sha256.clone(),
            command_plan_sha256: self.command_plan_sha256.clone(),
            argv_sha256: self.argv_sha256.clone(),
            executable_identity_sha256: self.executable_identity_sha256.clone(),
            target_identity_sha256: self.expected_target.target_sha256().to_owned(),
            target_generation: self.expected_target.generation(),
            issued_at_unix_ms,
            expires_at_unix_ms,
            expected_head_sha256: current_head.head_sha256().to_owned(),
            decision: HostEffectDecision::Authorize,
        })
    }

    #[cfg(test)]
    pub(super) fn host_scope(&self) -> &AcceptedHostScope {
        &self.scope
    }
}

pub(crate) struct RootPlanCustody {
    plan: Option<HostCommandPlan>,
}

impl std::fmt::Debug for RootPlanCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RootPlanCustody")
            .field("released", &self.is_released())
            .finish_non_exhaustive()
    }
}

impl RootPlanCustody {
    pub(in crate::distribution::host_effect) fn bind(
        plan: HostCommandPlan,
        accepted: &AcceptedHostEffect,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if plan.plan_sha256() != accepted.command_plan_sha256 {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::PlanSubstitution,
            ));
        }
        Ok(Self { plan: Some(plan) })
    }

    pub(crate) fn is_released(&self) -> bool {
        self.plan.is_none()
    }

    pub(super) fn plan_sha256(&self) -> Option<&str> {
        self.plan.as_ref().map(HostCommandPlan::plan_sha256)
    }

    /// Returns non-authoritative plan data for effect construction while this
    /// custody token remains live. Only `commit_release` consumes authority.
    pub(super) fn candidate_plan(&self) -> Result<HostCommandPlan, SupportedHostLifecycleError> {
        self.plan
            .clone()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))
    }

    pub(super) fn commit_release(&mut self) -> Result<(), SupportedHostLifecycleError> {
        self.plan
            .take()
            .map(|_| ())
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))
    }
}

#[derive(Serialize)]
struct PackageBinding<'a> {
    schema: &'static str,
    package: &'a PackageIdentity,
}

#[derive(Serialize)]
struct StateBinding<'a> {
    schema: &'static str,
    state: &'a AcceptedHostState,
}

#[derive(Serialize)]
struct RollbackBinding<'a> {
    schema: &'static str,
    rollback_state: &'a AcceptedHostState,
    policy: AcceptedRollbackPolicy,
}

#[derive(Serialize)]
struct ReconciliationBinding<'a> {
    schema: &'static str,
    expected_after: &'a AcceptedHostState,
    policy: AcceptedReconciliationPolicy,
}

#[derive(Serialize)]
struct RequiredCapabilityBinding<'a> {
    schema: &'static str,
    required: &'a [Capability],
}

#[derive(Serialize)]
struct ExternalRequestBinding<'a> {
    schema: &'static str,
    coordinator_binding_sha256: &'a str,
    session_issuance_sha256: &'a str,
    lifecycle_plan_sha256: &'a str,
    host_scope_sha256: &'a str,
    command_plan_sha256: &'a str,
    argv_sha256: &'a str,
    target_identity_sha256: &'a str,
}

fn session_issuance(
    coordinator_binding_sha256: &str,
    package_identity_sha256: &str,
    journey_binding_sha256: &str,
    lifecycle_plan_sha256: &str,
    host_scope_sha256: &str,
) -> Result<String, SupportedHostLifecycleError> {
    let mut nonce = [0_u8; SESSION_NONCE_BYTES];
    getrandom::fill(&mut nonce).map_err(|_| invalid())?;
    #[derive(Serialize)]
    struct Session<'a> {
        schema: &'static str,
        coordinator_binding_sha256: &'a str,
        package_identity_sha256: &'a str,
        journey_binding_sha256: &'a str,
        lifecycle_plan_sha256: &'a str,
        host_scope_sha256: &'a str,
        nonce_sha256: String,
    }
    let result = digest_json(&Session {
        schema: "harness-ultragoal.root-host-lifecycle-session.v1",
        coordinator_binding_sha256,
        package_identity_sha256,
        journey_binding_sha256,
        lifecycle_plan_sha256,
        host_scope_sha256,
        nonce_sha256: digest_bytes(&nonce),
    });
    nonce.fill(0);
    result
}

fn command_plan_sha256(
    package: &PackageIdentity,
    plan: &HostCommandPlan,
) -> Result<String, SupportedHostLifecycleError> {
    #[derive(Serialize)]
    struct Binding<'a> {
        schema: &'static str,
        package: &'a PackageIdentity,
        commands: &'a [crate::distribution::HostCommand],
    }
    digest_json(&Binding {
        schema: "harness-ultragoal.host-command-plan.v1",
        package,
        commands: plan.commands(),
    })
}

fn argv_sha256(plan: &HostCommandPlan) -> Result<String, SupportedHostLifecycleError> {
    #[derive(Serialize)]
    struct ExactArgv<'a> {
        schema: &'static str,
        commands: &'a [crate::distribution::HostCommand],
        shell: bool,
        inherited_environment: bool,
        output_limit_bytes: u64,
        timeout_required: bool,
    }
    if plan.commands().is_empty()
        || plan
            .commands()
            .iter()
            .any(|command| command.program() != "codex" || command.argv().is_empty())
    {
        return Err(invalid());
    }
    digest_json(&ExactArgv {
        schema: "harness-ultragoal.exact-host-command-argv.v1",
        commands: plan.commands(),
        shell: false,
        inherited_environment: false,
        output_limit_bytes: 1024 * 1024,
        timeout_required: true,
    })
}

fn validate_name(value: &str) -> Result<(), SupportedHostLifecycleError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(invalid());
    }
    Ok(())
}

fn project_identity(value: &str) -> Result<String, SupportedHostLifecycleError> {
    let canonical = Path::new(value).canonicalize().map_err(|_| invalid())?;
    if Path::new(value) != canonical {
        // The argv path itself is effectful input. Reject aliases so a stable
        // target descriptor cannot be paired with a later-resolved symlink.
        return Err(invalid());
    }
    let metadata = std::fs::symlink_metadata(&canonical).map_err(|_| invalid())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(digest_bytes(
            format!(
                "{}\0{}\0{}",
                canonical.display(),
                metadata.dev(),
                metadata.ino()
            )
            .as_bytes(),
        ))
    }
    #[cfg(not(unix))]
    {
        Ok(digest_bytes(
            format!("{}\0{}", canonical.display(), metadata.len()).as_bytes(),
        ))
    }
}

fn digest_json(value: &impl Serialize) -> Result<String, SupportedHostLifecycleError> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| invalid())
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn invalid() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::InvalidAcceptedIdentity)
}
