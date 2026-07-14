fn cas_or_interrupted(
    store: &dyn DurableMigrationStore,
    current: &MigrationOperation,
    next: &MigrationOperation,
) -> Result<MigrationOperation, ProductMigrationError> {
    match store.compare_and_swap(
        current.operation_id(),
        current.revision(),
        current.journal_sha256(),
        next,
    ) {
        Ok(stored) => {
            if stored != *next || !stored.validate_shape() {
                Err(ProductMigrationError::new(
                    "migration-product-store-substituted-journal",
                ))
            } else {
                Ok(stored)
            }
        }
        Err(_) => Err(ProductMigrationError::new(
            "migration-product-operation-interrupted",
        )),
    }
}

fn verify_operation(
    operation: &MigrationOperation,
    plan: &ProductMigrationPlan,
    request: &ReservationRequest,
) -> Result<(), ProductMigrationError> {
    if !operation.validate_shape()
        || operation.operation_id != request.operation_id
        || operation.authorization_id != request.authorization.authorization_id
        || operation.authorization != request.authorization
        || operation.reservation_sha256 != request.reservation_sha256
        || operation.plan_sha256 != plan.plan_sha256()
        || operation.input_binding != *plan.input_binding()
        || operation.semantic_keys != request.semantic_keys
        || operation.effects != plan.effects()
    {
        return Err(ProductMigrationError::new(
            "migration-product-reservation-substituted",
        ));
    }
    Ok(())
}

fn operation_boundary_seals_valid(
    operation: &MigrationOperation,
    plan: &ProductMigrationPlan,
    authority: &dyn ApplyAuthorizationAuthority,
) -> bool {
    match plan.compatibility_boundary_binding() {
        None => {
            operation
                .authorization
                .compatibility_boundary_observation
                .is_none()
                && operation.last_boundary_observation.is_none()
                && operation.pending_effect_permit.is_none()
        }
        Some(binding) => {
            binding.initial_observation().seal_verified_by(authority)
                && operation
                    .authorization
                    .compatibility_boundary_observation
                    .as_ref()
                    .is_some_and(|observation| observation.seal_verified_by(authority))
                && operation
                    .last_boundary_observation
                    .as_ref()
                    .is_some_and(|observation| observation.seal_verified_by(authority))
                && operation
                    .pending_effect_permit
                    .as_ref()
                    .is_none_or(|permit| permit.boundary_observation.seal_verified_by(authority))
        }
    }
}

fn validate_authorization(
    plan: &ProductMigrationPlan,
    record: &AuthorizationRecord,
    authority: &dyn ApplyAuthorizationAuthority,
) -> Result<(), ProductMigrationError> {
    let effect_set_sha256 = effect_set_digest(plan.effects());
    let expected_binding = authorization_binding(AuthorizationBinding {
        principal_id: &record.principal_id,
        authority_id: &record.authority_id,
        session_id: &record.authority_session_id,
        nonce_sha256: &record.nonce_sha256,
        issued_at_unix_ms: record.issued_at_unix_ms,
        expires_at_unix_ms: record.expires_at_unix_ms,
        input: plan.input_binding(),
        plan_sha256: plan.plan_sha256(),
        effect_set_sha256: &effect_set_sha256,
        compatibility_boundary_binding_sha256: record
            .compatibility_boundary_binding_sha256
            .as_deref(),
        compatibility_boundary_observation_sha256: record
            .compatibility_boundary_observation
            .as_ref()
            .map(CompatibilityBoundaryObservation::observation_sha256),
    });
    let compatibility_boundary_valid = match (
        plan.compatibility_boundary_binding(),
        record.compatibility_boundary_binding_sha256.as_deref(),
        record.compatibility_boundary_observation.as_ref(),
    ) {
        (Some(binding), Some(binding_sha256), Some(observation)) => {
            binding.binding_sha256() == binding_sha256
                && binding.initial_observation().seal_verified_by(authority)
                && observation.seal_verified_by(authority)
                && binding.observation_follows_source_history(
                    observation,
                    binding.initial_observation(),
                )
                && binding.all_effects_open_at(plan.effects(), observation)
        }
        (None, None, None) => true,
        _ => false,
    };
    let (bound_input, bound_plan) = authority.current_binding();
    if !record.validate_shape()
        || record.input_binding != *plan.input_binding()
        || record.plan_sha256 != plan.plan_sha256()
        || record.effect_set_sha256 != effect_set_digest(plan.effects())
        || !compatibility_boundary_valid
        || record.binding_sha256 != expected_binding
        || authority.principal_id() != record.principal_id
        || authority.authority_id() != record.authority_id
        || authority.session_id() != record.authority_session_id
        || authority.nonce_sha256() != record.nonce_sha256
        || authority.issued_at_unix_ms() != record.issued_at_unix_ms
        || authority.expires_at_unix_ms() != record.expires_at_unix_ms
        || bound_input != plan.input_binding().binding_sha256()
        || bound_plan != plan.plan_sha256()
        || !valid_window(
            record.issued_at_unix_ms,
            record.expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        || !authority.verify_seal(&record.binding_sha256, &record.seal_sha256)
    {
        return Err(ProductMigrationError::new(
            "migration-product-authorization-stale-or-rebound",
        ));
    }
    Ok(())
}

struct AuthorizationBinding<'a> {
    principal_id: &'a str,
    authority_id: &'a str,
    session_id: &'a str,
    nonce_sha256: &'a str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    input: &'a MigrationInputBinding,
    plan_sha256: &'a str,
    effect_set_sha256: &'a str,
    compatibility_boundary_binding_sha256: Option<&'a str>,
    compatibility_boundary_observation_sha256: Option<&'a str>,
}

fn authorization_binding(binding: AuthorizationBinding<'_>) -> String {
    let AuthorizationBinding {
        principal_id,
        authority_id,
        session_id,
        nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
        input,
        plan_sha256,
        effect_set_sha256,
        compatibility_boundary_binding_sha256,
        compatibility_boundary_observation_sha256,
    } = binding;
    digest(
        format!(
            "migration-apply-authorization-binding-v2|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            principal_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            input.binding_sha256(),
            plan_sha256,
            effect_set_sha256,
            compatibility_boundary_binding_sha256.unwrap_or("not-applicable"),
            compatibility_boundary_observation_sha256.unwrap_or("not-applicable"),
        )
        .as_bytes(),
    )
}
