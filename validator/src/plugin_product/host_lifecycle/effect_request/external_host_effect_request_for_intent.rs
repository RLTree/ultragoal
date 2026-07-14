impl ExternalHostEffectRequest {
    pub(super) fn for_intent(
        package: &PackageIdentity,
        binding: &JourneyBinding,
        lifecycle: &LifecyclePlan,
        issuance: Arc<SessionIssuance>,
        capability_gate: Arc<HostEffectCapabilityGate>,
        host_scope: Arc<BoundHostScope>,
        intent: LifecycleIntent,
    ) -> Result<Option<Self>, HostLifecycleError> {
        if matches!(
            intent,
            LifecycleIntent::RepeatUse | LifecycleIntent::IdempotentReinstall
        ) {
            return Ok(None);
        }
        if issuance.lifecycle() != lifecycle
            || issuance.binding_sha256() != binding.binding_sha256()
            || capability_gate.intent() != intent
        {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::InvalidBinding,
            ));
        }
        let scope = host_scope.authority();
        let remove = intent == LifecycleIntent::UninstallTeardown;
        let plan = match (scope, remove) {
            (HostScopeAuthority::Personal { marketplace }, false) => {
                HostCommandPlan::personal_install(package, marketplace)
            }
            (HostScopeAuthority::Personal { marketplace }, true) => {
                HostCommandPlan::personal_remove(package, marketplace)
            }
            (
                HostScopeAuthority::Repository {
                    repository_root,
                    marketplace,
                },
                false,
            ) => HostCommandPlan::repository_install(package, repository_root, marketplace),
            (HostScopeAuthority::Repository { marketplace, .. }, true) => {
                HostCommandPlan::repository_remove(package, marketplace)
            }
        }
        .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::InvalidBinding))?;
        let request_sha256 = digest_request(
            intent,
            binding,
            lifecycle,
            &issuance,
            &capability_gate,
            &host_scope,
            &plan,
        );
        Ok(Some(Self {
            intent,
            binding_sha256: binding.binding_sha256().to_owned(),
            session_issuance_sha256: issuance.issuance_sha256().to_owned(),
            host_scope_sha256: host_scope.scope_sha256().to_owned(),
            capability_gate_sha256: capability_gate.gate_sha256().to_owned(),
            lifecycle_plan_id: lifecycle.plan_id.clone(),
            plan_sha256: plan.plan_sha256().to_owned(),
            request_sha256,
            plan,
            issuance,
            capability_gate,
            host_scope,
        }))
    }

    pub const fn intent(&self) -> LifecycleIntent {
        self.intent
    }

    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    pub fn lifecycle_plan_id(&self) -> &str {
        &self.lifecycle_plan_id
    }

    pub fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    pub fn capability_gate_sha256(&self) -> &str {
        &self.capability_gate_sha256
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub(super) fn consume_for(
        self,
        expected: &Arc<SessionIssuance>,
        expected_gate: &Arc<HostEffectCapabilityGate>,
        expected_scope: &Arc<BoundHostScope>,
    ) -> Result<PreparedExternalHostEffect, HostLifecycleError> {
        if !same_issuance(&self.issuance, expected)
            || !same_gate(&self.capability_gate, expected_gate)
            || !same_scope(&self.host_scope, expected_scope)
            || self.binding_sha256 != expected.binding_sha256()
            || self.session_issuance_sha256 != expected.issuance_sha256()
            || self.lifecycle_plan_id != expected.lifecycle().plan_id
            || self.capability_gate_sha256 != expected_gate.gate_sha256()
            || self.host_scope_sha256 != expected_scope.scope_sha256()
        {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ExternalEffectSessionMismatch,
            ));
        }
        if let Err(error) = expected_scope.revalidate() {
            expected.reject();
            return Err(error);
        }
        if let Err(error) = expected_gate.require_supported() {
            expected.reject();
            return Err(error);
        }
        expected.consume_request()?;
        Ok(PreparedExternalHostEffect {
            intent: self.intent,
            binding_sha256: self.binding_sha256,
            session_issuance_sha256: self.session_issuance_sha256,
            host_scope_sha256: self.host_scope_sha256,
            capability_gate_sha256: self.capability_gate_sha256,
            request_sha256: self.request_sha256,
            plan: self.plan,
            issuance: self.issuance,
            capability_gate: self.capability_gate,
            host_scope: self.host_scope,
        })
    }
}

impl std::fmt::Debug for ExternalHostEffectRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExternalHostEffectRequest")
            .field("intent", &self.intent)
            .field("binding_sha256", &self.binding_sha256)
            .field("session_issuance_sha256", &self.session_issuance_sha256)
            .field("host_scope_sha256", &self.host_scope_sha256)
            .field("capability_gate_sha256", &self.capability_gate_sha256)
            .field("lifecycle_plan_id", &self.lifecycle_plan_id)
            .field("plan_sha256", &self.plan_sha256)
            .field("request_sha256", &self.request_sha256)
            .finish_non_exhaustive()
    }
}
