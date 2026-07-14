pub(crate) fn apply_product_plan(
    plan: &ProductMigrationPlan,
    authorization: &ApplyAuthorization,
    source: &mut dyn MigrationInputSource,
    authority: &dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<ApplyOutcome, ProductMigrationError> {
    plan.validate()?;
    validate_authorization(plan, &authorization.record, authority)?;
    let current = source.capture()?;
    let exact = derive_product_plan_from_bound_observation(
        &current,
        plan.compatibility_boundary_binding()
            .map(|binding| binding.initial_observation()),
    )?;
    if !exact_input_matches(plan.input_binding(), &current)
        || exact.plan_sha256() != plan.plan_sha256()
    {
        return Err(ProductMigrationError::new(
            "migration-product-apply-input-stale",
        ));
    }
    source.revalidate(plan.input_binding(), &[])?;
    let latest_boundary_observation = capture_open_plan_boundary_observation(
        plan,
        authority,
        authorization
            .record
            .compatibility_boundary_observation
            .as_ref(),
    )?;
    let request = ReservationRequest::issue(plan, &authorization.record)?;
    let initial = MigrationOperation::reserved(&request, plan, latest_boundary_observation)?;
    let reservation = store
        .reserve_once(&request, &initial)
        .map_err(|_| ProductMigrationError::new("migration-product-reservation-refused"))?;
    let (operation, repeat) = match reservation {
        ReservationResult::Created(operation) if operation == initial => (operation, false),
        ReservationResult::Created(_) => {
            return Err(ProductMigrationError::new(
                "migration-product-reservation-substituted",
            ));
        }
        ReservationResult::Existing(operation) => (operation, true),
    };
    verify_operation(&operation, plan, &request)?;
    if !operation_boundary_seals_valid(&operation, plan, authority) {
        return Err(ProductMigrationError::new(
            "migration-product-reservation-substituted",
        ));
    }
    drive_operation(operation, plan, source, authority, store, effects, repeat)
}

pub(crate) fn recover_product_operation(
    operation_id: &str,
    plan: &ProductMigrationPlan,
    source: &mut dyn MigrationInputSource,
    boundary_authority: &dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<ApplyOutcome, ProductMigrationError> {
    if !valid_sha256(operation_id) {
        return Err(ProductMigrationError::new(
            "migration-product-recovery-id-invalid",
        ));
    }
    plan.validate()?;
    let operation = store
        .load_operation(operation_id)
        .map_err(|_| ProductMigrationError::new("migration-product-recovery-store-failed"))?
        .ok_or_else(|| ProductMigrationError::new("migration-product-recovery-unknown"))?;
    if !operation.validate_shape()
        || operation.operation_id != operation_id
        || operation.plan_sha256 != plan.plan_sha256()
        || operation.input_binding != *plan.input_binding()
        || operation.effects != plan.effects()
        || operation.authorization.plan_sha256 != plan.plan_sha256()
        || operation.authorization.effect_set_sha256 != effect_set_digest(plan.effects())
        || operation
            .authorization
            .compatibility_boundary_binding_sha256
            .as_deref()
            != plan
                .compatibility_boundary_binding()
                .map(|binding| binding.binding_sha256())
        || !operation_boundary_seals_valid(&operation, plan, boundary_authority)
        || boundary_authority.principal_id() != operation.authorization.principal_id
        || boundary_authority.authority_id() != operation.authorization.authority_id
        || boundary_authority.session_id() != operation.authorization.authority_session_id
        || boundary_authority.nonce_sha256() != operation.authorization.nonce_sha256
        || !boundary_authority.verify_seal(
            &operation.authorization.binding_sha256,
            &operation.authorization.seal_sha256,
        )
    {
        return Err(ProductMigrationError::new(
            "migration-product-recovery-substituted",
        ));
    }
    capture_plan_boundary_observation_allow_crossed(
        plan,
        boundary_authority,
        operation.last_boundary_observation.as_ref(),
    )?;
    drive_operation(
        operation,
        plan,
        source,
        boundary_authority,
        store,
        effects,
        true,
    )
}
