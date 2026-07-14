impl DestructiveAuthorization {
    pub(crate) fn issue<R: RetirementReviewAuthority, A: DestructiveEffectAuthority>(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        review: &RetirementReview,
        review_authority: &R,
        authority: &mut A,
    ) -> Result<Self, MigrationError> {
        plan.verify_current(current)?;
        let target = plan
            .targets
            .iter()
            .find(|target| target.target_id == target_id)
            .ok_or_else(|| MigrationError::new("migration-retirement-target-unknown"))?;
        if review.od009_decision != Od009Decision::RequestPhysicalDeletion
            || !retirement_review_is_current(plan, target, current, review, review_authority)
        {
            return Err(MigrationError::new(
                "migration-destructive-authorization-issuance-refused",
            ));
        }

        let authority_id = authority.authority_id().to_owned();
        let principal_id = authority.principal_id().to_owned();
        let session_id = authority.session_id().to_owned();
        let nonce_sha256 = authority.nonce_sha256().to_owned();
        let issued_at_unix_ms = authority.issued_at_unix_ms();
        let expires_at_unix_ms = authority.expires_at_unix_ms();
        let now_unix_ms = authority.now_unix_ms();
        let effect_scope_sha256 = authority.effect_scope_sha256().to_owned();
        let (context, candidate, catalog, inventory) = authority.current_binding();
        let context = context.to_owned();
        let candidate = candidate.to_owned();
        let catalog = catalog.to_owned();
        let inventory = inventory.to_owned();

        if !valid_identifier(&authority_id)
            || !valid_identifier(&principal_id)
            || authority_id == principal_id
            || authority_id == review.authority_id
            || authority_id == review.reviewer_id
            || principal_id == review.authority_id
            || principal_id == review.reviewer_id
            || retirement_principal_conflicts(plan, target, &authority_id)
            || retirement_principal_conflicts(plan, target, &principal_id)
            || !valid_sha256(&session_id)
            || session_id == current.read_session_id
            || session_id == review.session_id
            || !valid_sha256(&nonce_sha256)
            || nonce_sha256 == review.nonce_sha256
            || !valid_authority_window(issued_at_unix_ms, expires_at_unix_ms, now_unix_ms)
            || effect_scope_sha256 != review.effect_scope_sha256
            || (context, candidate, catalog, inventory)
                != (
                    current.live_context_id.clone(),
                    current.candidate_id.clone(),
                    current.catalog_id.clone(),
                    current.inventory_sha256.clone(),
                )
        {
            return Err(MigrationError::new(
                "migration-destructive-authorization-issuance-refused",
            ));
        }

        let target_sha256 = digest(target.digest_fragment().as_bytes());
        let binding_sha256 = destructive_authorization_binding(DestructiveAuthorizationBinding {
            plan,
            target,
            current,
            review,
            principal_id: &principal_id,
            authority_id: &authority_id,
            session_id: &session_id,
            nonce_sha256: &nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            effect_scope_sha256: &effect_scope_sha256,
        });
        let attestation_sha256 = authority.issue_attestation(&binding_sha256)?;
        let authorization_id = digest(
            format!("destructive-authorization|{binding_sha256}|{attestation_sha256}").as_bytes(),
        );
        if !valid_sha256(&attestation_sha256)
            || authority.authority_id() != authority_id
            || authority.principal_id() != principal_id
            || authority.session_id() != session_id
            || authority.nonce_sha256() != nonce_sha256
            || authority.issued_at_unix_ms() != issued_at_unix_ms
            || authority.expires_at_unix_ms() != expires_at_unix_ms
            || authority.effect_scope_sha256() != effect_scope_sha256
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
                "migration-destructive-authorization-issuance-refused",
            ));
        }

        Ok(Self {
            principal_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            live_context_id: current.live_context_id.clone(),
            candidate_id: current.candidate_id.clone(),
            catalog_id: current.catalog_id.clone(),
            inventory_sha256: current.inventory_sha256.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            target_id: target.target_id.clone(),
            target_sha256,
            review_id: review.review_id.clone(),
            effect_scope_sha256,
            binding_sha256,
            authorization_id,
            attestation_sha256,
        })
    }

    #[cfg(test)]
    pub(crate) fn substitute_effect_scope_for_test(&mut self, scope: impl Into<String>) {
        self.effect_scope_sha256 = scope.into();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementStatus {
    Blocked,
    NonAuthoritativePreservationCandidate,
    DestructiveRetirementCandidate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RetirementDecision {
    pub target_id: String,
    pub live_context_id: String,
    pub candidate_id: String,
    pub reviewer_id: String,
    pub review_id: String,
    pub destructive_authorization_id: Option<String>,
    pub status: RetirementStatus,
    pub reasons: Vec<String>,
    pub plan_sha256: String,
    pub claim_ceiling: String,
}
