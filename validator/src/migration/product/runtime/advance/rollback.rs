fn advance_rollback_operation(
    operation: &MigrationOperation,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<MigrationOperation, ProductMigrationError> {
    let Some(index) = operation.rollback_effect_index else {
        return Err(ProductMigrationError::new(
            "migration-product-rollback-journal-invalid",
        ));
    };
    let effect = operation.effects[index].clone();
    let applied_permit_sha256 = operation
        .applied_effect_permit_sha256
        .get(index)
        .and_then(|permit| permit.as_deref());
    match effects.observe(&effect) {
        Ok(observation) if observation.matches(effect.before(), None) => {
            persist_rollback_completion(operation, store)
        }
        Ok(observation) if observation.matches(effect.after(), applied_permit_sha256) => {
            match effects.rollback(&operation.operation_id, &effect) {
                Ok(rolled_back) if rolled_back.matches(effect.before(), None) => {
                    match effects.observe(&effect) {
                        Ok(confirmed) if confirmed.matches(effect.before(), None) => {
                            persist_rollback_completion(operation, store)
                        }
                        _ => persist_ambiguous(operation, store),
                    }
                }
                _ => persist_ambiguous(operation, store),
            }
        }
        _ => persist_ambiguous(operation, store),
    }
}

fn persist_rollback_completion(
    operation: &MigrationOperation,
    store: &dyn DurableMigrationStore,
) -> Result<MigrationOperation, ProductMigrationError> {
    let next = operation.rollback_completed()?;
    cas_or_interrupted(store, operation, &next)
}
