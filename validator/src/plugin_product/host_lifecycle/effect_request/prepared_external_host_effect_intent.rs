impl PreparedExternalHostEffect {
    pub const fn intent(&self) -> LifecycleIntent {
        self.intent
    }

    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    pub fn capability_gate_sha256(&self) -> &str {
        &self.capability_gate_sha256
    }

    pub fn into_plan(self) -> Result<HostCommandPlan, HostLifecycleError> {
        if let Err(error) = self.host_scope.revalidate() {
            self.issuance.reject();
            return Err(error);
        }
        if let Err(error) = self.capability_gate.require_supported() {
            self.issuance.reject();
            return Err(error);
        }
        self.issuance.release_plan()?;
        Ok(self.plan)
    }
}

impl std::fmt::Debug for PreparedExternalHostEffect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedExternalHostEffect")
            .field("intent", &self.intent)
            .field("binding_sha256", &self.binding_sha256)
            .field("session_issuance_sha256", &self.session_issuance_sha256)
            .field("host_scope_sha256", &self.host_scope_sha256)
            .field("capability_gate_sha256", &self.capability_gate_sha256)
            .field("request_sha256", &self.request_sha256)
            .field("plan_sha256", &self.plan.plan_sha256())
            .finish_non_exhaustive()
    }
}

fn digest_request(
    intent: LifecycleIntent,
    binding: &JourneyBinding,
    lifecycle: &LifecyclePlan,
    issuance: &SessionIssuance,
    capability_gate: &HostEffectCapabilityGate,
    host_scope: &BoundHostScope,
    plan: &HostCommandPlan,
) -> String {
    #[derive(Serialize)]
    struct Request<'a> {
        schema: &'static str,
        intent: LifecycleIntent,
        binding_sha256: &'a str,
        session_issuance_sha256: &'a str,
        host_scope_sha256: &'a str,
        capability_gate_sha256: &'a str,
        lifecycle_plan_id: &'a str,
        lifecycle_authorization_sha256: &'a str,
        plan_sha256: &'a str,
    }
    let bytes = serde_json::to_vec(&Request {
        schema: "harness-ultragoal.external-host-effect-request.v1",
        intent,
        binding_sha256: binding.binding_sha256(),
        session_issuance_sha256: issuance.issuance_sha256(),
        host_scope_sha256: host_scope.scope_sha256(),
        capability_gate_sha256: capability_gate.gate_sha256(),
        lifecycle_plan_id: &lifecycle.plan_id,
        lifecycle_authorization_sha256: &lifecycle.authorization_sha256,
        plan_sha256: plan.plan_sha256(),
    })
    .expect("external host effect request is serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}
