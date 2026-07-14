fn drive_operation(
    mut operation: MigrationOperation,
    plan: &ProductMigrationPlan,
    source: &mut dyn MigrationInputSource,
    boundary_authority: &dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
    repeat: bool,
) -> Result<ApplyOutcome, ProductMigrationError> {
    for _ in 0..(plan.effects().len().saturating_mul(6).saturating_add(12)) {
        if !operation.validate_shape() {
            return Err(ProductMigrationError::new(
                "migration-product-journal-mutated",
            ));
        }
        operation = match operation.phase {
            JournalPhase::TerminalApplied
            | JournalPhase::TerminalRolledBack
            | JournalPhase::Ambiguous => {
                return Ok(ApplyOutcome::from_operation(&operation, repeat, None));
            }
            JournalPhase::Reserved => {
                advance_reserved_operation(&operation, source, store, effects)?
            }
            JournalPhase::EffectIntent => advance_effect_intent_operation(
                &operation,
                plan,
                boundary_authority,
                store,
                effects,
            )?,
            JournalPhase::RollingBack => advance_rollback_operation(&operation, store, effects)?,
            JournalPhase::EffectsApplied => {
                advance_applied_operation(&operation, source, store, effects)?
            }
        };
    }
    Ok(ApplyOutcome::from_operation(
        &operation,
        repeat,
        Some("migration-product-step-bound-exhausted"),
    ))
}
