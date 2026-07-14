fn advance_effect_intent_operation(
    operation: &MigrationOperation,
    plan: &ProductMigrationPlan,
    boundary_authority: &dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<MigrationOperation, ProductMigrationError> {
    let effect = operation.effects[operation.next_effect_index].clone();
    let pending_permit_sha256 = operation
        .pending_effect_permit
        .as_ref()
        .map(|permit| permit.permit_sha256.as_str());
    match effects.observe(&effect) {
        Ok(observation)
            if observation.matches(effect.after(), pending_permit_sha256)
                && (effect.disposition() != PlanDisposition::AdoptCompatibility
                    || pending_permit_sha256.is_some()) =>
        {
            persist_effect_completion(operation, store)
        }
        Ok(observation) if observation.matches(effect.before(), None) => {
            let authorized = authorize_compatibility_effect(
                operation,
                plan,
                &effect,
                boundary_authority,
                store,
            )?;
            if authorized.phase != JournalPhase::EffectIntent {
                return Ok(authorized);
            }
            apply_current_effect(&authorized, &effect, store, effects)
        }
        _ => persist_ambiguous(operation, store),
    }
}

fn authorize_compatibility_effect(
    operation: &MigrationOperation,
    plan: &ProductMigrationPlan,
    effect: &PlannedMigrationEffect,
    boundary_authority: &dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
) -> Result<MigrationOperation, ProductMigrationError> {
    if effect.disposition() != PlanDisposition::AdoptCompatibility {
        return Ok(operation.clone());
    }
    let observation =
        match capture_open_effect_boundary_observation(plan, operation, effect, boundary_authority)
        {
            Ok(observation) => observation,
            Err(error)
                if error.code() == "migration-product-compatibility-boundary-crossed"
                    && !operation.applied_effect_ids.is_empty() =>
            {
                let rollback = operation.begin_rollback()?;
                return cas_or_interrupted(store, operation, &rollback);
            }
            Err(error) => return Err(error),
        };
    let binding = plan.compatibility_boundary_binding().ok_or_else(|| {
        ProductMigrationError::new("migration-product-compatibility-boundary-binding-missing")
    })?;
    let permit = CompatibilityEffectPermitRecord::issue(
        &operation.operation_id,
        &operation.authorization_id,
        &operation.plan_sha256,
        effect,
        binding.binding_sha256(),
        observation,
    )?;
    let authorized = operation.authorize_current_effect(permit)?;
    cas_or_interrupted(store, operation, &authorized)
}

fn apply_current_effect(
    operation: &MigrationOperation,
    effect: &PlannedMigrationEffect,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<MigrationOperation, ProductMigrationError> {
    let permit_sha256 = operation
        .pending_effect_permit
        .as_ref()
        .map(|permit| permit.permit_sha256.as_str());
    match effects.apply(&operation.operation_id, effect, permit_sha256) {
        Ok(_) => reconcile_successful_effect(operation, effect, permit_sha256, store, effects),
        Err(fault) if !fault.ambiguous => {
            reconcile_failed_effect(operation, effect, permit_sha256, store, effects)
        }
        Err(_) => persist_ambiguous(operation, store),
    }
}

fn reconcile_successful_effect(
    operation: &MigrationOperation,
    effect: &PlannedMigrationEffect,
    permit_sha256: Option<&str>,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<MigrationOperation, ProductMigrationError> {
    match effects.observe(effect) {
        Ok(confirmed) if confirmed.matches(effect.after(), permit_sha256) => {
            persist_effect_completion(operation, store)
        }
        Ok(confirmed) if confirmed.matches(effect.before(), None) => {
            let rollback = operation.begin_rollback()?;
            cas_or_interrupted(store, operation, &rollback)
        }
        _ => persist_ambiguous(operation, store),
    }
}

fn reconcile_failed_effect(
    operation: &MigrationOperation,
    effect: &PlannedMigrationEffect,
    permit_sha256: Option<&str>,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<MigrationOperation, ProductMigrationError> {
    match effects.observe(effect) {
        Ok(confirmed) if confirmed.matches(effect.before(), None) => {
            let rollback = operation.begin_rollback()?;
            cas_or_interrupted(store, operation, &rollback)
        }
        Ok(confirmed) if confirmed.matches(effect.after(), permit_sha256) => {
            let rollback = operation.current_effect_applied_then_begin_rollback()?;
            cas_or_interrupted(store, operation, &rollback)
        }
        _ => persist_ambiguous(operation, store),
    }
}

fn persist_effect_completion(
    operation: &MigrationOperation,
    store: &dyn DurableMigrationStore,
) -> Result<MigrationOperation, ProductMigrationError> {
    let next = operation.effect_completed()?;
    cas_or_interrupted(store, operation, &next)
}

fn persist_ambiguous(
    operation: &MigrationOperation,
    store: &dyn DurableMigrationStore,
) -> Result<MigrationOperation, ProductMigrationError> {
    let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
    cas_or_interrupted(store, operation, &ambiguous)
}
