fn compatibility_effect_permit_digest(
    operation_id: &str,
    authorization_id: &str,
    plan_sha256: &str,
    effect: &PlannedMigrationEffect,
    boundary_binding_sha256: &str,
    observation: &CompatibilityBoundaryObservation,
) -> String {
    digest(
        format!(
            "migration-compatibility-effect-permit-v1|{}|{}|{}|{}|{}|{}|{}",
            operation_id,
            authorization_id,
            plan_sha256,
            effect.effect_id(),
            effect
                .compatibility_prerequisites_sha256()
                .unwrap_or("missing"),
            boundary_binding_sha256,
            observation.observation_sha256(),
        )
        .as_bytes(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MigrationOperation {
    schema_version: String,
    operation_id: String,
    authorization_id: String,
    authorization: AuthorizationRecord,
    reservation_sha256: String,
    plan_sha256: String,
    input_binding: MigrationInputBinding,
    semantic_keys: Vec<String>,
    effects: Vec<PlannedMigrationEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_boundary_observation: Option<CompatibilityBoundaryObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_effect_permit: Option<CompatibilityEffectPermitRecord>,
    phase: JournalPhase,
    next_effect_index: usize,
    applied_effect_ids: Vec<String>,
    applied_effect_permit_sha256: Vec<Option<String>>,
    rollback_effect_index: Option<usize>,
    terminal_proof_sha256: Option<String>,
    revision: u64,
    journal_sha256: String,
}
