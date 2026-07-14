pub(crate) fn issue_apply_authorization(
    plan: &ProductMigrationPlan,
    source: &mut dyn MigrationInputSource,
    authority: &mut dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
) -> Result<ApplyAuthorization, ProductMigrationError> {
    plan.validate()?;
    if plan.effects().is_empty() {
        return Err(ProductMigrationError::new(
            "migration-product-plan-has-no-adopted-effects",
        ));
    }
    let current = source.capture()?;
    current.validate()?;
    let exact = derive_product_plan_from_bound_observation(
        &current,
        plan.compatibility_boundary_binding()
            .map(|binding| binding.initial_observation()),
    )?;
    if !exact_input_matches(plan.input_binding(), &current)
        || exact.plan_sha256() != plan.plan_sha256()
    {
        return Err(ProductMigrationError::new(
            "migration-product-plan-stale-or-substituted",
        ));
    }
    source.revalidate(plan.input_binding(), &[])?;
    let compatibility_boundary_observation =
        capture_open_plan_boundary_observation(plan, authority, None)?;

    let principal_id = authority.principal_id().to_owned();
    let authority_id = authority.authority_id().to_owned();
    let authority_session_id = authority.session_id().to_owned();
    let nonce_sha256 = authority.nonce_sha256().to_owned();
    let issued_at_unix_ms = authority.issued_at_unix_ms();
    let expires_at_unix_ms = authority.expires_at_unix_ms();
    let (bound_input, bound_plan) = authority.current_binding();
    if !valid_identifier(&principal_id)
        || !valid_identifier(&authority_id)
        || principal_id == authority_id
        || !valid_sha256(&authority_session_id)
        || authority_session_id == plan.input_binding().read_session_id()
        || !valid_sha256(&nonce_sha256)
        || !valid_window(
            issued_at_unix_ms,
            expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        || bound_input != plan.input_binding().binding_sha256()
        || bound_plan != plan.plan_sha256()
    {
        return Err(ProductMigrationError::new(
            "migration-product-authorization-issuer-refused",
        ));
    }
    let effect_set_sha256 = effect_set_digest(plan.effects());
    let binding_sha256 = authorization_binding(AuthorizationBinding {
        principal_id: &principal_id,
        authority_id: &authority_id,
        session_id: &authority_session_id,
        nonce_sha256: &nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
        input: plan.input_binding(),
        plan_sha256: plan.plan_sha256(),
        effect_set_sha256: &effect_set_sha256,
        compatibility_boundary_binding_sha256: plan
            .compatibility_boundary_binding()
            .map(|binding| binding.binding_sha256()),
        compatibility_boundary_observation_sha256: compatibility_boundary_observation
            .as_ref()
            .map(CompatibilityBoundaryObservation::observation_sha256),
    });
    let seal_sha256 = authority.seal(&binding_sha256)?;
    let (current_input, current_plan) = authority.current_binding();
    if !valid_sha256(&seal_sha256)
        || !authority.verify_seal(&binding_sha256, &seal_sha256)
        || authority.principal_id() != principal_id
        || authority.authority_id() != authority_id
        || authority.session_id() != authority_session_id
        || authority.nonce_sha256() != nonce_sha256
        || authority.issued_at_unix_ms() != issued_at_unix_ms
        || authority.expires_at_unix_ms() != expires_at_unix_ms
        || current_input != plan.input_binding().binding_sha256()
        || current_plan != plan.plan_sha256()
        || !valid_window(
            issued_at_unix_ms,
            expires_at_unix_ms,
            authority.now_unix_ms(),
        )
    {
        return Err(ProductMigrationError::new(
            "migration-product-authorization-seal-refused",
        ));
    }
    let post_seal_boundary_observation = capture_open_plan_boundary_observation(
        plan,
        authority,
        compatibility_boundary_observation.as_ref(),
    )?;
    if compatibility_boundary_observation.is_some() != post_seal_boundary_observation.is_some() {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-authorization-refused",
        ));
    }
    let authorization_id = digest(
        format!(
            "migration-apply-authorization-v2|{}|{}",
            binding_sha256, seal_sha256
        )
        .as_bytes(),
    );
    let record = AuthorizationRecord {
        schema_version: "MigrationApplyAuthorizationRecord-v2".to_owned(),
        authorization_id,
        principal_id,
        authority_id,
        authority_session_id,
        nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
        input_binding: plan.input_binding().clone(),
        plan_sha256: plan.plan_sha256().to_owned(),
        effect_set_sha256,
        compatibility_boundary_binding_sha256: plan
            .compatibility_boundary_binding()
            .map(|binding| binding.binding_sha256().to_owned()),
        compatibility_boundary_observation,
        binding_sha256,
        seal_sha256,
    };
    if !record.validate_shape() {
        return Err(ProductMigrationError::new(
            "migration-product-authorization-invalid",
        ));
    }
    store
        .register_authorization(&record)
        .map_err(|_| ProductMigrationError::new("migration-product-authorization-store-refused"))?;
    Ok(ApplyAuthorization { record })
}
