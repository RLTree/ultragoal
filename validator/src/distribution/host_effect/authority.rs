use super::HostEffectReservation;
use getrandom::fill;
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 32;
const MAX_PERMIT_TTL_MS: u64 = 5 * 60 * 1000;
const PERMIT_SCHEMA: &str = "harness-ultragoal.host-effect-permit.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectDecision {
    Authorize,
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
}

impl Drop for HostEffectAuthority {
    fn drop(&mut self) {
        zeroize(&mut self.key);
    }
}

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
    if digests.into_iter().any(|row| !is_digest(row))
        || !valid_id(&binding.lifecycle_intent)
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

fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn zeroize<const N: usize>(bytes: &mut [u8; N]) {
    for byte in bytes {
        // SAFETY: each pointer comes from a unique mutable slice element. The
        // volatile store and fence prevent elision of this best-effort wipe.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

const fn authority_error(id: HostEffectAuthorityErrorId) -> HostEffectAuthorityError {
    HostEffectAuthorityError { id }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn binding() -> HostEffectPermitBinding {
        HostEffectPermitBinding {
            context_id: d('1'),
            candidate_id: d('2'),
            package_identity_sha256: d('3'),
            journey_binding_sha256: d('4'),
            session_issuance_sha256: d('5'),
            lifecycle_plan_sha256: d('6'),
            lifecycle_intent: "install".to_owned(),
            expected_pre_state_sha256: d('7'),
            expected_post_state_sha256: d('8'),
            rollback_policy_sha256: d('9'),
            reconciliation_policy_sha256: d('a'),
            host_scope_sha256: d('b'),
            host_capability_sha256: d('c'),
            required_capabilities_sha256: d('d'),
            external_request_sha256: d('e'),
            command_plan_sha256: d('f'),
            argv_sha256: d('1'),
            executable_identity_sha256: d('2'),
            target_identity_sha256: d('3'),
            target_generation: 1,
            issued_at_unix_ms: 1_000,
            expires_at_unix_ms: 2_000,
            expected_head_sha256: d('4'),
            decision: HostEffectDecision::Authorize,
        }
    }

    fn issue_error(
        result: Result<(HostEffectPermit, HostEffectReservation), HostEffectAuthorityError>,
    ) -> HostEffectAuthorityErrorId {
        match result {
            Ok(_) => panic!("permit unexpectedly issued"),
            Err(error) => error.id(),
        }
    }

    #[test]
    fn issued_permit_verifies_only_under_exact_authority_and_binding() {
        let authority =
            HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned())
                .unwrap();
        let (permit, reservation) = authority.issue(binding()).unwrap();
        authority.verify(&permit, 1_500).unwrap();
        assert_eq!(reservation.permit_id(), permit.permit_id());
        assert_eq!(
            reservation.semantic_key_sha256(),
            permit.semantic_key_sha256()
        );
    }

    #[test]
    fn field_mutation_wrong_authority_and_time_fail_closed() {
        let authority =
            HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned())
                .unwrap();
        let (mut permit, _) = authority.issue(binding()).unwrap();
        permit.binding.candidate_id = d('a');
        assert_eq!(
            authority.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::InvalidBinding
        );

        let (permit, _) = authority.issue(binding()).unwrap();
        assert_eq!(
            authority.verify(&permit, 999).unwrap_err().id(),
            HostEffectAuthorityErrorId::NotYetValid
        );
        assert_eq!(
            authority.verify(&permit, 2_001).unwrap_err().id(),
            HostEffectAuthorityErrorId::Expired
        );
        let other =
            HostEffectAuthority::generate("other-root".to_owned(), "host-ledger".to_owned())
                .unwrap();
        assert_eq!(
            other.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::WrongAuthority
        );
    }

    #[test]
    fn nonce_and_permit_are_unique_while_semantic_key_is_stable() {
        let authority =
            HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned())
                .unwrap();
        let (left, _) = authority.issue(binding()).unwrap();
        let (right, _) = authority.issue(binding()).unwrap();
        assert_ne!(left.nonce_sha256(), right.nonce_sha256());
        assert_ne!(left.permit_id(), right.permit_id());
        assert_eq!(left.semantic_key_sha256(), right.semantic_key_sha256());

        let mut reissued = binding();
        reissued.session_issuance_sha256 = d('9');
        reissued.external_request_sha256 = d('8');
        reissued.executable_identity_sha256 = d('7');
        reissued.target_identity_sha256 = d('6');
        reissued.target_generation = 2;
        reissued.issued_at_unix_ms = 2_000;
        reissued.expires_at_unix_ms = 3_000;
        let (reissued, _) = authority.issue(reissued).unwrap();
        assert_eq!(left.semantic_key_sha256(), reissued.semantic_key_sha256());
    }

    #[test]
    fn refusal_and_overlong_ttl_never_issue() {
        let authority =
            HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned())
                .unwrap();
        let mut refused = binding();
        refused.decision = HostEffectDecision::Refuse;
        assert_eq!(
            issue_error(authority.issue(refused)),
            HostEffectAuthorityErrorId::Refused
        );
        let mut stale = binding();
        stale.expires_at_unix_ms = stale.issued_at_unix_ms + MAX_PERMIT_TTL_MS + 1;
        assert_eq!(
            issue_error(authority.issue(stale)),
            HostEffectAuthorityErrorId::InvalidBinding
        );
    }

    #[test]
    fn every_binding_and_permit_field_mutation_fails_closed() {
        let authority =
            HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned())
                .unwrap();
        let mutations: &[fn(&mut HostEffectPermitBinding)] = &[
            |row| row.context_id = d('a'),
            |row| row.candidate_id = d('a'),
            |row| row.package_identity_sha256 = d('a'),
            |row| row.journey_binding_sha256 = d('a'),
            |row| row.session_issuance_sha256 = d('a'),
            |row| row.lifecycle_plan_sha256 = d('a'),
            |row| row.lifecycle_intent = "remove".to_owned(),
            |row| row.expected_pre_state_sha256 = d('a'),
            |row| row.expected_post_state_sha256 = d('a'),
            |row| row.rollback_policy_sha256 = d('b'),
            |row| row.reconciliation_policy_sha256 = d('b'),
            |row| row.host_scope_sha256 = d('a'),
            |row| row.host_capability_sha256 = d('a'),
            |row| row.required_capabilities_sha256 = d('a'),
            |row| row.external_request_sha256 = d('a'),
            |row| row.command_plan_sha256 = d('a'),
            |row| row.argv_sha256 = d('a'),
            |row| row.executable_identity_sha256 = d('a'),
            |row| row.target_identity_sha256 = d('a'),
            |row| row.target_generation += 1,
            |row| row.issued_at_unix_ms += 1,
            |row| row.expires_at_unix_ms += 1,
            |row| row.expected_head_sha256 = d('a'),
            |row| row.decision = HostEffectDecision::Refuse,
        ];
        for mutate in mutations {
            let (mut permit, _) = authority.issue(binding()).unwrap();
            mutate(&mut permit.binding);
            assert!(authority.verify(&permit, 1_500).is_err());
        }

        let (mut permit, _) = authority.issue(binding()).unwrap();
        permit.nonce[0] ^= 1;
        assert_eq!(
            authority.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::InvalidBinding
        );
        let (mut permit, _) = authority.issue(binding()).unwrap();
        permit.nonce_sha256 = d('a');
        assert_eq!(
            authority.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::InvalidBinding
        );
        let (mut permit, _) = authority.issue(binding()).unwrap();
        permit.binding_sha256 = d('a');
        assert_eq!(
            authority.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::InvalidBinding
        );
        let (mut permit, _) = authority.issue(binding()).unwrap();
        permit.semantic_key_sha256 = d('a');
        assert_eq!(
            authority.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::InvalidBinding
        );
        let (mut permit, _) = authority.issue(binding()).unwrap();
        permit.tag[0] ^= 1;
        assert_eq!(
            authority.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::InvalidMac
        );
        let (mut permit, _) = authority.issue(binding()).unwrap();
        permit.permit_id = d('a');
        assert_eq!(
            authority.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::InvalidBinding
        );
        let identity_mutations: &[fn(&mut HostEffectPermit)] = &[
            |row: &mut HostEffectPermit| row.issuer_id = "other-root".to_owned(),
            |row: &mut HostEffectPermit| row.ledger_id = "other-ledger".to_owned(),
            |row: &mut HostEffectPermit| row.key_id = d('a'),
        ];
        for mutate_identity in identity_mutations {
            let (mut permit, _) = authority.issue(binding()).unwrap();
            mutate_identity(&mut permit);
            assert_eq!(
                authority.verify(&permit, 1_500).unwrap_err().id(),
                HostEffectAuthorityErrorId::WrongAuthority
            );
        }
    }

    #[test]
    fn semantic_key_is_retry_stable_but_changes_for_distinct_effects() {
        let authority =
            HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned())
                .unwrap();
        let (baseline, _) = authority.issue(binding()).unwrap();
        let baseline = baseline.semantic_key_sha256().to_owned();

        let retry_mutations: &[fn(&mut HostEffectPermitBinding)] = &[
            |row| row.session_issuance_sha256 = d('a'),
            |row| row.external_request_sha256 = d('a'),
            |row| row.executable_identity_sha256 = d('a'),
            |row| row.target_identity_sha256 = d('a'),
            |row| row.target_generation += 1,
            |row| row.issued_at_unix_ms += 1,
            |row| row.expires_at_unix_ms += 1,
            |row| row.expected_head_sha256 = d('a'),
        ];
        for mutate in retry_mutations {
            let mut row = binding();
            mutate(&mut row);
            let (permit, _) = authority.issue(row).unwrap();
            assert_eq!(permit.semantic_key_sha256(), baseline.as_str());
        }

        let effect_mutations: &[fn(&mut HostEffectPermitBinding)] = &[
            |row| row.context_id = d('a'),
            |row| row.candidate_id = d('a'),
            |row| row.package_identity_sha256 = d('a'),
            |row| row.journey_binding_sha256 = d('a'),
            |row| row.lifecycle_plan_sha256 = d('a'),
            |row| row.lifecycle_intent = "remove".to_owned(),
            |row| row.host_scope_sha256 = d('a'),
            |row| row.command_plan_sha256 = d('a'),
        ];
        for mutate in effect_mutations {
            let mut row = binding();
            mutate(&mut row);
            let (permit, _) = authority.issue(row).unwrap();
            assert_ne!(permit.semantic_key_sha256(), baseline.as_str());
        }
    }
}
