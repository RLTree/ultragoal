impl AcceptedHostEffect {
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
        let lifecycle_intent = serde_json::to_string(&self.lifecycle_record.permit_join().1)
            .map_err(|_| invalid())?
            .trim_matches('"')
            .to_owned();
        Ok(HostEffectPermitBinding {
            context_id: self.package.source().context_id().to_owned(),
            candidate_id: self.package.source().candidate_id().to_owned(),
            package_identity_sha256: self.package_identity_sha256.clone(),
            journey_binding_sha256: self.journey_binding_sha256.clone(),
            session_issuance_sha256: self.session_issuance_sha256.clone(),
            // The permit carries the durable lifecycle-plan identity. The
            // accepted projection remains bound through the session and
            // external-request digests below, but it cannot replace the
            // record that the ledger will persist with the permit.
            lifecycle_plan_sha256: self.lifecycle_record.permit_join().0.to_owned(),
            lifecycle_intent,
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
            lifecycle_record: Some(self.lifecycle_record.clone()),
            lifecycle_record_sha256: Some(
                serde_json::to_vec(&self.lifecycle_record)
                    .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
                    .map_err(|_| invalid())?,
            ),
            decision: HostEffectDecision::Authorize,
        })
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
