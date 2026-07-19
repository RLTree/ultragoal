struct DestructiveAuthorizationCurrentness<'a> {
    plan: &'a MigrationPlan,
    target: &'a RetirementTarget,
    current: &'a MigrationInventory,
    review: &'a RetirementReview,
    authorization: &'a DestructiveAuthorization,
    review_authority: &'a dyn RetirementReviewAuthority,
    authority: &'a dyn DestructiveEffectAuthority,
}

fn destructive_authorization_is_current(request: DestructiveAuthorizationCurrentness<'_>) -> bool {
    let DestructiveAuthorizationCurrentness {
        plan,
        target,
        current,
        review,
        authorization,
        review_authority,
        authority,
    } = request;
    if !retirement_review_is_current(plan, target, current, review, review_authority)
        || review.od009_decision != Od009Decision::RequestPhysicalDeletion
    {
        return false;
    }
    let Ok(effect_scope_sha256) = retirement_effect_scope(plan, target, current) else {
        return false;
    };
    let expected_binding = destructive_authorization_binding(DestructiveAuthorizationBinding {
        plan,
        target,
        current,
        review,
        principal_id: &authorization.principal_id,
        authority_id: &authorization.authority_id,
        session_id: &authorization.session_id,
        nonce_sha256: &authorization.nonce_sha256,
        issued_at_unix_ms: authorization.issued_at_unix_ms,
        expires_at_unix_ms: authorization.expires_at_unix_ms,
        effect_scope_sha256: &effect_scope_sha256,
    });
    let expected_authorization_id = digest(
        format!(
            "destructive-authorization|{}|{}",
            expected_binding, authorization.attestation_sha256
        )
        .as_bytes(),
    );
    valid_identifier(&authorization.principal_id)
        && valid_identifier(&authorization.authority_id)
        && authorization.principal_id != authorization.authority_id
        && authorization.principal_id != review.reviewer_id
        && authorization.principal_id != review.authority_id
        && authorization.authority_id != review.reviewer_id
        && authorization.authority_id != review.authority_id
        && !retirement_principal_conflicts(plan, target, &authorization.principal_id)
        && !retirement_principal_conflicts(plan, target, &authorization.authority_id)
        && valid_sha256(&authorization.session_id)
        && authorization.session_id != current.read_session_id
        && authorization.session_id != review.session_id
        && valid_sha256(&authorization.nonce_sha256)
        && authorization.nonce_sha256 != review.nonce_sha256
        && valid_authority_window(
            authorization.issued_at_unix_ms,
            authorization.expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        && authorization.live_context_id == current.live_context_id
        && authorization.candidate_id == current.candidate_id
        && authorization.catalog_id == current.catalog_id
        && authorization.inventory_sha256 == current.inventory_sha256
        && authorization.plan_sha256 == plan.plan_sha256
        && authorization.target_id == target.target_id
        && authorization.target_sha256 == digest(target.digest_fragment().as_bytes())
        && authorization.review_id == review.review_id
        && authorization.effect_scope_sha256 == effect_scope_sha256
        && valid_sha256(&authorization.attestation_sha256)
        && authorization.binding_sha256 == expected_binding
        && authorization.authorization_id == expected_authorization_id
        && authority.authority_id() == authorization.authority_id
        && authority.principal_id() == authorization.principal_id
        && authority.session_id() == authorization.session_id
        && authority.nonce_sha256() == authorization.nonce_sha256
        && authority.issued_at_unix_ms() == authorization.issued_at_unix_ms
        && authority.expires_at_unix_ms() == authorization.expires_at_unix_ms
        && authority.effect_scope_sha256() == authorization.effect_scope_sha256
        && authority.current_binding()
            == (
                current.live_context_id.as_str(),
                current.candidate_id.as_str(),
                current.catalog_id.as_str(),
                current.inventory_sha256.as_str(),
            )
}

fn inventory_digest(
    context: &str,
    candidate: &str,
    catalog: &str,
    session: &str,
    surfaces: &[InventorySurface],
) -> String {
    let rows = surfaces
        .iter()
        .map(InventorySurface::digest_fragment)
        .collect::<Vec<_>>()
        .join("\n");
    digest(format!("{context}|{candidate}|{catalog}|{session}|{rows}").as_bytes())
}

fn plan_digest(
    inventory: &MigrationInventory,
    routes: &[CompatibilityRoute],
    targets: &[RetirementTarget],
) -> String {
    let route_rows = routes
        .iter()
        .map(CompatibilityRoute::digest_fragment)
        .collect::<Vec<_>>()
        .join("\n");
    let target_rows = targets
        .iter()
        .map(RetirementTarget::digest_fragment)
        .collect::<Vec<_>>()
        .join("\n");
    digest(
        format!(
            "{}|{}|{}",
            inventory.inventory_sha256, route_rows, target_rows
        )
        .as_bytes(),
    )
}
