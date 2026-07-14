fn retirement_review_is_current(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    current: &MigrationInventory,
    review: &RetirementReview,
    authority: &dyn RetirementReviewAuthority,
) -> bool {
    let Ok(effect_scope_sha256) = retirement_effect_scope(plan, target, current) else {
        return false;
    };
    let expected_binding = retirement_review_binding(RetirementReviewBinding {
        plan,
        target,
        current,
        reviewer_id: &review.reviewer_id,
        authority_id: &review.authority_id,
        session_id: &review.session_id,
        nonce_sha256: &review.nonce_sha256,
        issued_at_unix_ms: review.issued_at_unix_ms,
        expires_at_unix_ms: review.expires_at_unix_ms,
        decision: review.od009_decision,
        effect_scope_sha256: &effect_scope_sha256,
    });
    let expected_review_id = digest(
        format!(
            "retirement-review|{}|{}",
            expected_binding, review.attestation_sha256
        )
        .as_bytes(),
    );
    valid_identifier(&review.reviewer_id)
        && valid_identifier(&review.authority_id)
        && review.reviewer_id != review.authority_id
        && !retirement_principal_conflicts(plan, target, &review.reviewer_id)
        && !retirement_principal_conflicts(plan, target, &review.authority_id)
        && valid_sha256(&review.session_id)
        && review.session_id != current.read_session_id
        && valid_sha256(&review.nonce_sha256)
        && valid_authority_window(
            review.issued_at_unix_ms,
            review.expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        && plan.verify_current(current).is_ok()
        && review.live_context_id == current.live_context_id
        && review.candidate_id == current.candidate_id
        && review.catalog_id == current.catalog_id
        && review.read_session_id == current.read_session_id
        && review.inventory_sha256 == current.inventory_sha256
        && review.plan_sha256 == plan.plan_sha256
        && review.target_id == target.target_id
        && review.target_sha256 == digest(target.digest_fragment().as_bytes())
        && review.effect_scope_sha256 == effect_scope_sha256
        && valid_sha256(&review.attestation_sha256)
        && review.binding_sha256 == expected_binding
        && review.review_id == expected_review_id
        && authority.authority_id() == review.authority_id
        && authority.reviewer_id() == review.reviewer_id
        && authority.session_id() == review.session_id
        && authority.nonce_sha256() == review.nonce_sha256
        && authority.issued_at_unix_ms() == review.issued_at_unix_ms
        && authority.expires_at_unix_ms() == review.expires_at_unix_ms
        && authority.od009_decision() == review.od009_decision
        && authority.current_binding()
            == (
                current.live_context_id.as_str(),
                current.candidate_id.as_str(),
                current.catalog_id.as_str(),
                current.inventory_sha256.as_str(),
            )
}

struct DestructiveAuthorizationBinding<'a> {
    plan: &'a MigrationPlan,
    target: &'a RetirementTarget,
    current: &'a MigrationInventory,
    review: &'a RetirementReview,
    principal_id: &'a str,
    authority_id: &'a str,
    session_id: &'a str,
    nonce_sha256: &'a str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    effect_scope_sha256: &'a str,
}

fn destructive_authorization_binding(binding: DestructiveAuthorizationBinding<'_>) -> String {
    let DestructiveAuthorizationBinding {
        plan,
        target,
        current,
        review,
        principal_id,
        authority_id,
        session_id,
        nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
        effect_scope_sha256,
    } = binding;
    digest(
        format!(
            "destructive-authorization-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            principal_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            current.live_context_id,
            current.candidate_id,
            current.catalog_id,
            current.inventory_sha256,
            plan.plan_sha256,
            target.target_id,
            target.digest_fragment(),
            review.review_id,
            effect_scope_sha256,
        )
        .as_bytes(),
    )
}
