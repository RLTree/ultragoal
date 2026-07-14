pub(in crate::distribution::host_effect) struct HostEffectAcceptanceRequest<'a> {
    pub package: PackageIdentity,
    pub journey: JourneyBinding,
    pub host: HostCapabilityDeclaration,
    pub lifecycle: AcceptedLifecyclePlan,
    pub scope: AcceptedHostScope,
    pub plan: &'a HostCommandPlan,
    pub executable: &'a PinnedHostExecutable,
    pub expected_target: ObservedTargetIdentity,
    pub expected_head: HostEffectLedgerHead,
}

impl AcceptedHostEffect {
    pub(super) fn accept(
        coordinator_binding_sha256: String,
        request: HostEffectAcceptanceRequest<'_>,
    ) -> Result<Self, SupportedHostLifecycleError> {
        let HostEffectAcceptanceRequest {
            package,
            journey,
            host,
            lifecycle,
            scope,
            plan,
            executable,
            expected_target,
            expected_head,
        } = request;
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
}
