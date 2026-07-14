impl MigrationPlan {
    /// Confirms only that an inspection DTO is the current exact projection.
    /// It never imports or reconstructs authority from serialized bytes.
    pub fn verify_projection(
        &self,
        projection: &MigrationPlanProjection,
    ) -> Result<(), MigrationError> {
        if &self.projection() != projection {
            return Err(MigrationError::new("migration-plan-projection-substituted"));
        }
        Ok(())
    }

    fn replacement_evidence(
        &self,
        target: &RetirementTarget,
    ) -> Option<&ConsumedReplacementEvidence> {
        self.replacement_ledger
            .get(&target.target_id)
            .filter(|evidence| {
                evidence.route_id == target.route_id
                    && evidence.source_id == target.source_id
                    && evidence.canonical_target_id == target.canonical_target_id
                    && replacement_summary_commitment(evidence) == target.replacement_summary_sha256
            })
    }

    fn private_replacement_ledger_is_current(&self) -> bool {
        self.replacement_ledger.by_target_id.len() == self.targets.len()
            && self
                .targets
                .iter()
                .all(|target| self.replacement_evidence(target).is_some())
    }

    #[cfg(test)]
    pub(crate) fn substitute_replacement_consumption_for_test(
        &mut self,
        consumption_sha256: impl Into<String>,
    ) {
        if let Some(evidence) = self.replacement_ledger.by_target_id.values_mut().next() {
            evidence.consumption_sha256 = consumption_sha256.into();
        }
    }

    #[cfg(test)]
    pub(crate) fn inject_private_serialization_sentinels_for_test(&mut self) -> Vec<String> {
        self.replacement_ledger
            .by_target_id
            .values_mut()
            .next()
            .map(ConsumedReplacementEvidence::inject_serialization_sentinels_for_test)
            .unwrap_or_default()
    }
}

fn replacement_summary_commitment(evidence: &ConsumedReplacementEvidence) -> String {
    digest(
        format!(
            "replacement-plan-inspection-summary-v1|{}|{}|{}|{}",
            evidence.route_id,
            evidence.source_id,
            evidence.canonical_target_id,
            evidence.digest_fragment(),
        )
        .as_bytes(),
    )
}

fn migration_plan_projection_digest(
    plan_sha256: &str,
    context_commitment_sha256: &str,
    route_count: usize,
    targets: &[RetirementTargetProjection],
) -> String {
    let target_count = targets.len();
    let target_rows = targets
        .iter()
        .map(RetirementTargetProjection::digest_fragment)
        .collect::<Vec<_>>()
        .join("|");
    digest(
        format!(
            "migration-plan-projection-v1|{plan_sha256}|{context_commitment_sha256}|{route_count}|{target_count}|{target_rows}",
        )
        .as_bytes(),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Od009Decision {
    PreservePhysicalArtifact,
    RequestPhysicalDeletion,
}

pub(crate) trait RetirementReviewAuthority {
    fn authority_id(&self) -> &str;
    fn reviewer_id(&self) -> &str;
    fn session_id(&self) -> &str;
    fn nonce_sha256(&self) -> &str;
    fn issued_at_unix_ms(&self) -> u64;
    fn expires_at_unix_ms(&self) -> u64;
    fn now_unix_ms(&self) -> u64;
    fn current_binding(&self) -> (&str, &str, &str, &str);
    fn od009_decision(&self) -> Od009Decision;
    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, MigrationError>;
    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> bool;
}

pub(crate) trait DestructiveEffectAuthority {
    fn authority_id(&self) -> &str;
    fn principal_id(&self) -> &str;
    fn session_id(&self) -> &str;
    fn nonce_sha256(&self) -> &str;
    fn issued_at_unix_ms(&self) -> u64;
    fn expires_at_unix_ms(&self) -> u64;
    fn now_unix_ms(&self) -> u64;
    fn current_binding(&self) -> (&str, &str, &str, &str);
    fn effect_scope_sha256(&self) -> &str;
    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, MigrationError>;
    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        authorization_id: &str,
        attestation_sha256: &str,
    ) -> bool;
}

/// Opaque root-issued review of one exact plan and retirement target. It is
/// intentionally neither clonable nor deserializable and has no public
/// constructor.
#[derive(Eq, PartialEq)]
pub struct RetirementReview {
    reviewer_id: String,
    authority_id: String,
    session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    plan_sha256: String,
    target_id: String,
    target_sha256: String,
    effect_scope_sha256: String,
    od009_decision: Od009Decision,
    binding_sha256: String,
    review_id: String,
    attestation_sha256: String,
}

impl fmt::Debug for RetirementReview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetirementReview")
            .field("contents", &"<redacted>")
            .finish()
    }
}
