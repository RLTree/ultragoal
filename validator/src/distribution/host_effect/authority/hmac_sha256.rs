type HmacSha256 = Hmac<Sha256>;

use crate::plugin_product::lifecycle::HostLifecycleRecord;

const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 32;
const MAX_PERMIT_TTL_MS: u64 = 5 * 60 * 1000;
const PERMIT_SCHEMA: &str = "harness-ultragoal.host-effect-permit.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectDecision {
    Authorize,
    #[cfg(test)]
    Refuse,
}

/// Complete canonical permit input. Fields are visible only to descendants of
/// the root-owned host-effect module; callers elsewhere cannot construct it.
#[derive(Clone, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectPermitBinding {
    pub(in crate::distribution::host_effect) context_id: String,
    pub(in crate::distribution::host_effect) candidate_id: String,
    pub(in crate::distribution::host_effect) package_identity_sha256: String,
    pub(in crate::distribution::host_effect) journey_binding_sha256: String,
    pub(in crate::distribution::host_effect) session_issuance_sha256: String,
    pub(in crate::distribution::host_effect) lifecycle_plan_sha256: String,
    pub(in crate::distribution::host_effect) lifecycle_intent: String,
    pub(in crate::distribution::host_effect) expected_pre_state_sha256: String,
    pub(in crate::distribution::host_effect) expected_post_state_sha256: String,
    pub(in crate::distribution::host_effect) rollback_policy_sha256: String,
    pub(in crate::distribution::host_effect) reconciliation_policy_sha256: String,
    pub(in crate::distribution::host_effect) host_scope_sha256: String,
    pub(in crate::distribution::host_effect) host_capability_sha256: String,
    pub(in crate::distribution::host_effect) required_capabilities_sha256: String,
    pub(in crate::distribution::host_effect) external_request_sha256: String,
    pub(in crate::distribution::host_effect) command_plan_sha256: String,
    pub(in crate::distribution::host_effect) argv_sha256: String,
    pub(in crate::distribution::host_effect) executable_identity_sha256: String,
    pub(in crate::distribution::host_effect) target_identity_sha256: String,
    pub(in crate::distribution::host_effect) target_generation: u64,
    pub(in crate::distribution::host_effect) issued_at_unix_ms: u64,
    pub(in crate::distribution::host_effect) expires_at_unix_ms: u64,
    pub(in crate::distribution::host_effect) expected_head_sha256: String,
    pub(in crate::distribution::host_effect) lifecycle_record: Option<HostLifecycleRecord>,
    pub(in crate::distribution::host_effect) lifecycle_record_sha256: Option<String>,
    pub(in crate::distribution::host_effect) decision: HostEffectDecision,
}

/// Root-held symmetric authority. Key bytes are never exposed, cloned,
/// serialized, or included in Debug output.
pub(crate) struct HostEffectAuthority {
    issuer_id: String,
    ledger_id: String,
    key_id: String,
    key: [u8; KEY_BYTES],
}

/// Opaque, one-owner permit. The durable ledger must separately reserve its
/// nonce and semantic key before an executor may receive it.
pub(crate) struct HostEffectPermit {
    issuer_id: String,
    ledger_id: String,
    key_id: String,
    binding: HostEffectPermitBinding,
    nonce: [u8; NONCE_BYTES],
    nonce_sha256: String,
    binding_sha256: String,
    semantic_key_sha256: String,
    permit_id: String,
    tag: [u8; 32],
}

pub(crate) struct VerifiedHostEffectPermit {
    permit: HostEffectPermit,
}

impl HostEffectAuthority {
    pub(in crate::distribution::host_effect) fn generate(
        issuer_id: String,
        ledger_id: String,
    ) -> Result<Self, HostEffectAuthorityError> {
        if !valid_id(&issuer_id) || !valid_id(&ledger_id) {
            return Err(authority_error(HostEffectAuthorityErrorId::InvalidBinding));
        }
        let mut key = [0_u8; KEY_BYTES];
        fill(&mut key)
            .map_err(|_| authority_error(HostEffectAuthorityErrorId::RandomUnavailable))?;
        let key_id = digest(&key);
        Ok(Self {
            issuer_id,
            ledger_id,
            key_id,
            key,
        })
    }

    pub(in crate::distribution::host_effect) fn issue(
        &self,
        binding: HostEffectPermitBinding,
    ) -> Result<(HostEffectPermit, HostEffectReservation), HostEffectAuthorityError> {
        validate_binding(&binding)?;
        if binding.decision != HostEffectDecision::Authorize {
            return Err(authority_error(HostEffectAuthorityErrorId::Refused));
        }
        let mut nonce = [0_u8; NONCE_BYTES];
        fill(&mut nonce)
            .map_err(|_| authority_error(HostEffectAuthorityErrorId::RandomUnavailable))?;
        let binding_sha256 = digest_json(&binding)?;
        let semantic_key_sha256 = semantic_key(&binding)?;
        let nonce_sha256 = digest(&nonce);
        let preimage = permit_preimage(
            &self.issuer_id,
            &self.ledger_id,
            &self.key_id,
            &binding,
            &nonce,
        )?;
        let tag = sign(&self.key, &preimage)?;
        let permit_id = digest_parts(&[&preimage, &tag]);
        let permit = HostEffectPermit {
            issuer_id: self.issuer_id.clone(),
            ledger_id: self.ledger_id.clone(),
            key_id: self.key_id.clone(),
            binding,
            nonce,
            nonce_sha256,
            binding_sha256,
            semantic_key_sha256,
            permit_id,
            tag,
        };
        let reservation = HostEffectReservation::from_permit(&permit);
        Ok((permit, reservation))
    }

    pub(in crate::distribution::host_effect) fn verify(
        &self,
        permit: &HostEffectPermit,
        trusted_now_unix_ms: u64,
    ) -> Result<(), HostEffectAuthorityError> {
        if permit.issuer_id != self.issuer_id
            || permit.ledger_id != self.ledger_id
            || permit.key_id != self.key_id
        {
            return Err(authority_error(HostEffectAuthorityErrorId::WrongAuthority));
        }
        validate_binding(&permit.binding)?;
        if trusted_now_unix_ms < permit.binding.issued_at_unix_ms {
            return Err(authority_error(HostEffectAuthorityErrorId::NotYetValid));
        }
        if trusted_now_unix_ms > permit.binding.expires_at_unix_ms {
            return Err(authority_error(HostEffectAuthorityErrorId::Expired));
        }
        if digest(&permit.nonce) != permit.nonce_sha256
            || digest_json(&permit.binding)? != permit.binding_sha256
            || semantic_key(&permit.binding)? != permit.semantic_key_sha256
        {
            return Err(authority_error(HostEffectAuthorityErrorId::InvalidBinding));
        }
        let preimage = permit_preimage(
            &permit.issuer_id,
            &permit.ledger_id,
            &permit.key_id,
            &permit.binding,
            &permit.nonce,
        )?;
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .map_err(|_| authority_error(HostEffectAuthorityErrorId::InvalidMac))?;
        mac.update(&preimage);
        mac.verify_slice(&permit.tag)
            .map_err(|_| authority_error(HostEffectAuthorityErrorId::InvalidMac))?;
        if digest_parts(&[&preimage, &permit.tag]) != permit.permit_id {
            return Err(authority_error(HostEffectAuthorityErrorId::InvalidBinding));
        }
        Ok(())
    }

    #[cfg(not(test))]
    pub(in crate::distribution::host_effect) fn verify_at(
        &self,
        permit: HostEffectPermit,
        trusted_now_unix_ms: u64,
    ) -> Result<VerifiedHostEffectPermit, HostEffectAuthorityError> {
        self.verify(&permit, trusted_now_unix_ms)?;
        Ok(VerifiedHostEffectPermit { permit })
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn verify_current(
        &self,
        permit: HostEffectPermit,
    ) -> Result<VerifiedHostEffectPermit, HostEffectAuthorityError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| authority_error(HostEffectAuthorityErrorId::NotYetValid))?
            .as_millis()
            .try_into()
            .map_err(|_| authority_error(HostEffectAuthorityErrorId::Expired))?;
        self.verify(&permit, now)?;
        Ok(VerifiedHostEffectPermit { permit })
    }
}

impl Drop for HostEffectAuthority {
    fn drop(&mut self) {
        zeroize(&mut self.key);
    }
}
