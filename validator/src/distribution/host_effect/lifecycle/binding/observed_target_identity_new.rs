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
    lifecycle_record: crate::plugin_product::lifecycle::HostLifecycleRecord,
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
