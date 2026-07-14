fn authorization_digest(
    schema_version: &str,
    classification_sha256: &str,
    coordinator_binding_sha256: &str,
    ledger_head: &HostEffectLedgerHead,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    nonce_sha256: &str,
) -> Result<String, SupportedHostLifecycleError> {
    if schema_version != RECOVERY_AUTHORIZATION_SCHEMA
        || !is_digest(classification_sha256)
        || !is_digest(coordinator_binding_sha256)
        || !is_digest(ledger_head.head_sha256())
        || issued_at_unix_ms == 0
        || issued_at_unix_ms.checked_add(60_000) != Some(expires_at_unix_ms)
        || !is_digest(nonce_sha256)
    {
        return Err(recovery_authorization_required());
    }
    #[derive(Serialize)]
    struct Authorization<'a> {
        schema: &'a str,
        classification_sha256: &'a str,
        coordinator_binding_sha256: &'a str,
        ledger_head: &'a HostEffectLedgerHead,
        issued_at_unix_ms: u64,
        expires_at_unix_ms: u64,
        nonce_sha256: &'a str,
    }
    digest_json(&Authorization {
        schema: schema_version,
        classification_sha256,
        coordinator_binding_sha256,
        ledger_head,
        issued_at_unix_ms,
        expires_at_unix_ms,
        nonce_sha256,
    })
}

fn acknowledgement_digest(
    effect_identity_sha256: &str,
    publication_identity_sha256: &str,
    ledger_head: &HostEffectLedgerHead,
) -> Result<String, SupportedHostLifecycleError> {
    if !is_digest(effect_identity_sha256) || !is_digest(publication_identity_sha256) {
        return Err(recovery_unsafe());
    }
    #[derive(Serialize)]
    struct Acknowledgement<'a> {
        schema: &'static str,
        effect_identity_sha256: &'a str,
        publication_identity_sha256: &'a str,
        ledger_head: &'a HostEffectLedgerHead,
    }
    digest_json(&Acknowledgement {
        schema: "harness-ultragoal.publication-acknowledgement-identity.v1",
        effect_identity_sha256,
        publication_identity_sha256,
        ledger_head,
    })
}

fn digest_json(value: &impl Serialize) -> Result<String, SupportedHostLifecycleError> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::RecoveryUnsafe))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn recovery_authorization_required() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired)
}

fn recovery_unsafe() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::RecoveryUnsafe)
}
