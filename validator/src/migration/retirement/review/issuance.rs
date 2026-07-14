impl RetirementReview {
    pub(crate) fn issue<A: RetirementReviewAuthority>(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        authority: &mut A,
    ) -> Result<Self, MigrationError> {
        plan.verify_current(current)?;
        let target = plan
            .targets
            .iter()
            .find(|target| target.target_id == target_id)
            .ok_or_else(|| MigrationError::new("migration-retirement-target-unknown"))?;
        let authority_id = authority.authority_id().to_owned();
        let reviewer_id = authority.reviewer_id().to_owned();
        let session_id = authority.session_id().to_owned();
        let nonce_sha256 = authority.nonce_sha256().to_owned();
        let issued_at_unix_ms = authority.issued_at_unix_ms();
        let expires_at_unix_ms = authority.expires_at_unix_ms();
        let now_unix_ms = authority.now_unix_ms();
        let od009_decision = authority.od009_decision();
        let (context, candidate, catalog, inventory) = authority.current_binding();
        let context = context.to_owned();
        let candidate = candidate.to_owned();
        let catalog = catalog.to_owned();
        let inventory = inventory.to_owned();
        let target_sha256 = digest(target.digest_fragment().as_bytes());
        let effect_scope_sha256 = retirement_effect_scope(plan, target, current)?;

        if !valid_identifier(&authority_id)
            || !valid_identifier(&reviewer_id)
            || authority_id == reviewer_id
            || retirement_principal_conflicts(plan, target, &authority_id)
            || retirement_principal_conflicts(plan, target, &reviewer_id)
            || !valid_sha256(&session_id)
            || session_id == current.read_session_id
            || !valid_sha256(&nonce_sha256)
            || !valid_authority_window(issued_at_unix_ms, expires_at_unix_ms, now_unix_ms)
            || (context, candidate, catalog, inventory)
                != (
                    current.live_context_id.clone(),
                    current.candidate_id.clone(),
                    current.catalog_id.clone(),
                    current.inventory_sha256.clone(),
                )
        {
            return Err(MigrationError::new(
                "migration-retirement-review-issuance-refused",
            ));
        }

        let binding_sha256 = retirement_review_binding(RetirementReviewBinding {
            plan,
            target,
            current,
            reviewer_id: &reviewer_id,
            authority_id: &authority_id,
            session_id: &session_id,
            nonce_sha256: &nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            decision: od009_decision,
            effect_scope_sha256: &effect_scope_sha256,
        });
        let attestation_sha256 = authority.issue_attestation(&binding_sha256)?;
        let review_id =
            digest(format!("retirement-review|{binding_sha256}|{attestation_sha256}").as_bytes());
        if !valid_sha256(&attestation_sha256)
            || authority.authority_id() != authority_id
            || authority.reviewer_id() != reviewer_id
            || authority.session_id() != session_id
            || authority.nonce_sha256() != nonce_sha256
            || authority.issued_at_unix_ms() != issued_at_unix_ms
            || authority.expires_at_unix_ms() != expires_at_unix_ms
            || authority.od009_decision() != od009_decision
            || authority.current_binding()
                != (
                    current.live_context_id.as_str(),
                    current.candidate_id.as_str(),
                    current.catalog_id.as_str(),
                    current.inventory_sha256.as_str(),
                )
            || !valid_authority_window(
                issued_at_unix_ms,
                expires_at_unix_ms,
                authority.now_unix_ms(),
            )
        {
            return Err(MigrationError::new(
                "migration-retirement-review-issuance-refused",
            ));
        }

        Ok(Self {
            reviewer_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            live_context_id: current.live_context_id.clone(),
            candidate_id: current.candidate_id.clone(),
            catalog_id: current.catalog_id.clone(),
            read_session_id: current.read_session_id.clone(),
            inventory_sha256: current.inventory_sha256.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            target_id: target.target_id.clone(),
            target_sha256,
            effect_scope_sha256,
            od009_decision,
            binding_sha256,
            review_id,
            attestation_sha256,
        })
    }

    pub(crate) fn effect_scope_sha256(&self) -> &str {
        &self.effect_scope_sha256
    }

    #[cfg(test)]
    pub(crate) fn substitute_plan_for_test(&mut self, plan_sha256: impl Into<String>) {
        self.plan_sha256 = plan_sha256.into();
    }

    #[cfg(test)]
    pub(crate) fn substitute_target_for_test(&mut self, target_id: impl Into<String>) {
        self.target_id = target_id.into();
    }
}

/// Opaque authorization for the one exact physical effect scope selected by
/// an authenticated OD-009 deletion review. This token cannot perform effects.
#[derive(Eq, PartialEq)]
pub struct DestructiveAuthorization {
    principal_id: String,
    authority_id: String,
    session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    inventory_sha256: String,
    plan_sha256: String,
    target_id: String,
    target_sha256: String,
    review_id: String,
    effect_scope_sha256: String,
    binding_sha256: String,
    authorization_id: String,
    attestation_sha256: String,
}

impl fmt::Debug for DestructiveAuthorization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DestructiveAuthorization")
            .field("contents", &"<redacted>")
            .finish()
    }
}
