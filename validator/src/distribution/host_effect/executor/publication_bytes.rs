fn publication_bytes(
    effect_identity_sha256: &str,
    effect: &AuthorizedHostEffect,
    command_digests: &[CommandCaptureDigest],
    policy_sha256: &str,
    outcome_sha256: &str,
    completed_at_unix_ms: u64,
) -> Result<Vec<u8>, HostEffectExecutorFailure> {
    #[derive(Serialize)]
    struct Publication<'a> {
        schema_version: &'static str,
        effect_identity_sha256: &'a str,
        permit_id: &'a str,
        command_plan_sha256: &'a str,
        command_digests: &'a [CommandCaptureDigest],
        policy_sha256: &'a str,
        outcome_sha256: &'a str,
        completed_at_unix_ms: u64,
    }
    serde_json::to_vec(&Publication {
        schema_version: "SupportedHostEffectPublication-v1",
        effect_identity_sha256,
        permit_id: effect.permit().permit_id(),
        command_plan_sha256: effect.plan().plan_sha256(),
        command_digests,
        policy_sha256,
        outcome_sha256,
        completed_at_unix_ms,
    })
    .map_err(|_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io))
}

fn ledger_failure() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::LedgerSubstitution)
}

fn target_failure() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::TargetSubstitution)
}
