fn replacement_plan_ledger_is_current(
    plan: &MigrationPlan,
    current: &MigrationInventory,
    authority: &mut dyn ReplacementEvidenceAuthority,
) -> bool {
    if plan.verify_current(current).is_err() {
        return false;
    }
    for target in &plan.targets {
        let Some(route) = plan
            .routes
            .iter()
            .find(|route| route.route_id == target.route_id)
        else {
            return false;
        };
        let Some(evidence) = plan.replacement_evidence(target) else {
            return false;
        };
        if evidence
            .validate_with_authority(current, route, authority)
            .is_err()
        {
            return false;
        }
        let Ok(binding) = ReplacementLedgerBinding::issue(plan, target, current) else {
            return false;
        };
        if !authority.verify_plan_target_consumed(&binding)
            || evidence
                .validate_with_authority(current, route, authority)
                .is_err()
        {
            return false;
        }
    }
    true
}

fn valid_authority_window(issued_at: u64, expires_at: u64, now: u64) -> bool {
    issued_at <= now
        && now <= expires_at
        && issued_at < expires_at
        && expires_at.saturating_sub(issued_at) <= MAX_AUTHORIZATION_TTL_MS
}

fn retirement_principal_conflicts(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    principal_id: &str,
) -> bool {
    principal_id == target.owner_id
        || plan
            .replacement_evidence(target)
            .is_some_and(|evidence| principal_id == evidence.reviewer_id)
        || target
            .active_readers
            .iter()
            .chain(&target.active_writers)
            .any(|principal| principal == principal_id)
}

fn retirement_effect_scope(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    current: &MigrationInventory,
) -> Result<String, MigrationError> {
    let source = current
        .surfaces
        .iter()
        .find(|surface| surface.stable_id == target.source_id)
        .ok_or_else(|| MigrationError::new("migration-retirement-source-unknown"))?;
    Ok(digest(
        format!(
            "od-009-retirement-effect-v1|{}|{}|{}|{}|{}|{}|{}",
            current.live_context_id,
            current.candidate_id,
            current.catalog_id,
            current.inventory_sha256,
            plan.plan_sha256,
            target.digest_fragment(),
            source.digest_fragment(),
        )
        .as_bytes(),
    ))
}

struct RetirementReviewBinding<'a> {
    plan: &'a MigrationPlan,
    target: &'a RetirementTarget,
    current: &'a MigrationInventory,
    reviewer_id: &'a str,
    authority_id: &'a str,
    session_id: &'a str,
    nonce_sha256: &'a str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    decision: Od009Decision,
    effect_scope_sha256: &'a str,
}

fn retirement_review_binding(binding: RetirementReviewBinding<'_>) -> String {
    let RetirementReviewBinding {
        plan,
        target,
        current,
        reviewer_id,
        authority_id,
        session_id,
        nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
        decision,
        effect_scope_sha256,
    } = binding;
    digest(
        format!(
            "retirement-review-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}",
            reviewer_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            current.live_context_id,
            current.candidate_id,
            current.catalog_id,
            current.read_session_id,
            current.inventory_sha256,
            plan.plan_sha256,
            target.target_id,
            target.digest_fragment(),
            decision,
            effect_scope_sha256,
        )
        .as_bytes(),
    )
}
