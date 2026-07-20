impl HostEffectPermit {
    pub(in crate::distribution::host_effect) fn issuer_id(&self) -> &str {
        &self.issuer_id
    }

    pub(in crate::distribution::host_effect) fn ledger_id(&self) -> &str {
        &self.ledger_id
    }

    pub(in crate::distribution::host_effect) fn key_id(&self) -> &str {
        &self.key_id
    }

    pub(crate) fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub(crate) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(crate) fn semantic_key_sha256(&self) -> &str {
        &self.semantic_key_sha256
    }

    pub(crate) fn nonce_sha256(&self) -> &str {
        &self.nonce_sha256
    }

    pub(in crate::distribution::host_effect) fn binding(&self) -> &HostEffectPermitBinding {
        &self.binding
    }
}

impl VerifiedHostEffectPermit {
    pub(in crate::distribution::host_effect) fn binding(&self) -> &HostEffectPermitBinding {
        self.permit.binding()
    }

    pub(in crate::distribution::host_effect) fn into_permit(self) -> HostEffectPermit {
        self.permit
    }
}

impl Drop for HostEffectPermit {
    fn drop(&mut self) {
        zeroize(&mut self.nonce);
        zeroize(&mut self.tag);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostEffectAuthorityErrorId {
    InvalidBinding,
    RandomUnavailable,
    Refused,
    WrongAuthority,
    NotYetValid,
    Expired,
    InvalidMac,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HostEffectAuthorityError {
    id: HostEffectAuthorityErrorId,
}

impl HostEffectAuthorityError {
    pub(crate) const fn id(&self) -> HostEffectAuthorityErrorId {
        self.id
    }
}

fn validate_binding(binding: &HostEffectPermitBinding) -> Result<(), HostEffectAuthorityError> {
    let digests = [
        &binding.context_id,
        &binding.candidate_id,
        &binding.package_identity_sha256,
        &binding.journey_binding_sha256,
        &binding.session_issuance_sha256,
        &binding.lifecycle_plan_sha256,
        &binding.expected_pre_state_sha256,
        &binding.expected_post_state_sha256,
        &binding.rollback_policy_sha256,
        &binding.reconciliation_policy_sha256,
        &binding.host_scope_sha256,
        &binding.host_capability_sha256,
        &binding.required_capabilities_sha256,
        &binding.external_request_sha256,
        &binding.command_plan_sha256,
        &binding.argv_sha256,
        &binding.executable_identity_sha256,
        &binding.target_identity_sha256,
        &binding.expected_head_sha256,
    ];
    let lifecycle_join_is_valid = binding.lifecycle_record.as_ref().is_none_or(|record| {
        record.validate().is_ok()
            && binding.lifecycle_record_sha256.as_ref().is_some_and(
                |digest| matches!(digest_json(record), Ok(actual) if actual == *digest),
            )
            && record.permit_join().0 == binding.lifecycle_plan_sha256
            && serde_json::to_string(&record.permit_join().1)
                .ok()
                .is_some_and(|intent| intent.trim_matches('"') == binding.lifecycle_intent)
    });
    if digests.into_iter().any(|row| !is_digest(row))
        || !valid_id(&binding.lifecycle_intent)
        || (cfg!(not(test)) && binding.lifecycle_record.is_none())
        || !lifecycle_join_is_valid
        || binding.target_generation == 0
        || binding.issued_at_unix_ms >= binding.expires_at_unix_ms
        || binding
            .expires_at_unix_ms
            .saturating_sub(binding.issued_at_unix_ms)
            > MAX_PERMIT_TTL_MS
    {
        return Err(authority_error(HostEffectAuthorityErrorId::InvalidBinding));
    }
    Ok(())
}

fn permit_preimage(
    issuer_id: &str,
    ledger_id: &str,
    key_id: &str,
    binding: &HostEffectPermitBinding,
    nonce: &[u8; NONCE_BYTES],
) -> Result<Vec<u8>, HostEffectAuthorityError> {
    #[derive(Serialize)]
    struct Preimage<'a> {
        schema: &'static str,
        issuer_id: &'a str,
        ledger_id: &'a str,
        key_id: &'a str,
        binding: &'a HostEffectPermitBinding,
        nonce_sha256: String,
    }
    serde_json::to_vec(&Preimage {
        schema: PERMIT_SCHEMA,
        issuer_id,
        ledger_id,
        key_id,
        binding,
        nonce_sha256: digest(nonce),
    })
    .map_err(|_| authority_error(HostEffectAuthorityErrorId::InvalidBinding))
}

fn semantic_key(binding: &HostEffectPermitBinding) -> Result<String, HostEffectAuthorityError> {
    // This key names the logical effect independently of retry/session/time,
    // executable, and observed target generations. Those mutable dimensions
    // remain MAC-bound, but cannot be changed to evade duplicate-effect
    // reservation after a crash or ambiguous outcome.
    #[derive(Serialize)]
    struct SemanticKey<'a> {
        schema: &'static str,
        context_id: &'a str,
        candidate_id: &'a str,
        package_identity_sha256: &'a str,
        journey_binding_sha256: &'a str,
        lifecycle_plan_sha256: &'a str,
        lifecycle_intent: &'a str,
        host_scope_sha256: &'a str,
        command_plan_sha256: &'a str,
        decision: HostEffectDecision,
    }
    digest_json(&SemanticKey {
        schema: "harness-ultragoal.host-effect-semantic-key.v1",
        context_id: &binding.context_id,
        candidate_id: &binding.candidate_id,
        package_identity_sha256: &binding.package_identity_sha256,
        journey_binding_sha256: &binding.journey_binding_sha256,
        lifecycle_plan_sha256: &binding.lifecycle_plan_sha256,
        lifecycle_intent: &binding.lifecycle_intent,
        host_scope_sha256: &binding.host_scope_sha256,
        command_plan_sha256: &binding.command_plan_sha256,
        decision: binding.decision,
    })
}

fn sign(key: &[u8], preimage: &[u8]) -> Result<[u8; 32], HostEffectAuthorityError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| authority_error(HostEffectAuthorityErrorId::InvalidMac))?;
    mac.update(preimage);
    let bytes = mac.finalize().into_bytes();
    let mut tag = [0_u8; 32];
    tag.copy_from_slice(&bytes);
    Ok(tag)
}

fn digest_json(value: &impl Serialize) -> Result<String, HostEffectAuthorityError> {
    serde_json::to_vec(value)
        .map(|bytes| digest(&bytes))
        .map_err(|_| authority_error(HostEffectAuthorityErrorId::InvalidBinding))
}

fn digest(value: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(value))
}

fn digest_parts(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}
