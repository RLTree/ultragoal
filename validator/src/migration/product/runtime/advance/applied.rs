fn advance_applied_operation(
    operation: &MigrationOperation,
    source: &mut dyn MigrationInputSource,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<MigrationOperation, ProductMigrationError> {
    if source
        .revalidate(&operation.input_binding, &operation.applied_effect_ids)
        .is_err()
    {
        return persist_ambiguous(operation, store);
    }
    let mut observations = Vec::with_capacity(operation.effects.len());
    for (index, effect) in operation.effects.iter().enumerate() {
        let applied_permit_sha256 = operation
            .applied_effect_permit_sha256
            .get(index)
            .and_then(|permit| permit.as_deref());
        let Ok(observation) = effects.observe(effect) else {
            return persist_ambiguous(operation, store);
        };
        if !observation.matches(effect.after(), applied_permit_sha256)
            || retired_authority_remains_active(effect, &observation)
        {
            return persist_ambiguous(operation, store);
        }
        observations.push(observation.observation_sha256().to_owned());
    }
    let proof = digest(
        format!(
            "migration-product-terminal-semantic-proof-v1|{}|{}|{}",
            operation.operation_id,
            operation.plan_sha256,
            observations.join(",")
        )
        .as_bytes(),
    );
    let terminal = operation.terminal_applied(proof)?;
    cas_or_interrupted(store, operation, &terminal)
}

fn retired_authority_remains_active(
    effect: &PlannedMigrationEffect,
    observation: &EffectObservation,
) -> bool {
    effect.disposition() == PlanDisposition::RetireAuthority
        && (observation.authority().status() != super::super::SurfaceStatus::Retired
            || !observation.authority().active_readers().is_empty()
            || !observation.authority().active_writers().is_empty()
            || !observation.authority().public_routes().is_empty()
            || !observation.authority().generated_outputs().is_empty())
}
