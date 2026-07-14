fn advance_reserved_operation(
    operation: &MigrationOperation,
    source: &mut dyn MigrationInputSource,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<MigrationOperation, ProductMigrationError> {
    if source
        .revalidate(&operation.input_binding, &operation.applied_effect_ids)
        .is_err()
    {
        let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
        return cas_or_interrupted(store, operation, &ambiguous);
    }
    if operation.next_effect_index == operation.effects.len() {
        let next = operation.transition(JournalPhase::EffectsApplied)?;
        return cas_or_interrupted(store, operation, &next);
    }
    let effect = &operation.effects[operation.next_effect_index];
    let next = match effects.observe(effect) {
        Ok(observation) if observation.matches(effect.before(), None) => {
            operation.transition(JournalPhase::EffectIntent)?
        }
        Ok(_) | Err(_) => operation.transition(JournalPhase::Ambiguous)?,
    };
    cas_or_interrupted(store, operation, &next)
}
