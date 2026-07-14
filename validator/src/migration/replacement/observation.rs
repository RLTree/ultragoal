impl ReplacementObservation {
    fn capture<A: ReplacementEvidenceAuthority + ?Sized>(
        route: &CompatibilityRoute,
        authority: &A,
    ) -> Result<Self, MigrationError> {
        let (old_behavior_id, old_verdict, old_result_sha256) = authority
            .old_behavior(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?;
        let (new_behavior_id, new_verdict, new_result_sha256) = authority
            .new_behavior(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?;
        let (journey_execution_id, journey_verdict, journey_result_sha256) = authority
            .representative_journey(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?;
        let false_pass_control_results = authority
            .false_pass_control_results(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?
            .clone();
        let (rollback_execution_id, rollback_verdict, rollback_result_sha256) = authority
            .rollback_execution(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?;
        let observation = Self {
            old_behavior_id: old_behavior_id.to_owned(),
            old_verdict,
            old_result_sha256: old_result_sha256.to_owned(),
            new_behavior_id: new_behavior_id.to_owned(),
            new_verdict,
            new_result_sha256: new_result_sha256.to_owned(),
            journey_execution_id: journey_execution_id.to_owned(),
            journey_verdict,
            journey_result_sha256: journey_result_sha256.to_owned(),
            false_pass_control_results,
            rollback_execution_id: rollback_execution_id.to_owned(),
            rollback_verdict,
            rollback_result_sha256: rollback_result_sha256.to_owned(),
        };
        observation.validate(route)?;
        Ok(observation)
    }

    fn validate(&self, route: &CompatibilityRoute) -> Result<(), MigrationError> {
        if self.old_behavior_id != route.source_id
            || self.new_behavior_id != route.canonical_target_id
            || !valid_identifier(&self.journey_execution_id)
            || !valid_identifier(&self.rollback_execution_id)
            || self.old_verdict != EvidenceVerdict::Passed
            || self.new_verdict != EvidenceVerdict::Passed
            || self.journey_verdict != EvidenceVerdict::Passed
            || self.rollback_verdict != EvidenceVerdict::Passed
        {
            return Err(MigrationError::new(
                "migration-replacement-observation-invalid",
            ));
        }
        let expected_controls = REQUIRED_FALSE_PASS_CONTROLS
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        if self
            .false_pass_control_results
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != expected_controls
            || self
                .false_pass_control_results
                .values()
                .any(|(verdict, digest)| {
                    *verdict != EvidenceVerdict::CausalFailure || !valid_sha256(digest)
                })
            || [
                self.old_result_sha256.as_str(),
                self.new_result_sha256.as_str(),
                self.journey_result_sha256.as_str(),
                self.rollback_result_sha256.as_str(),
            ]
            .iter()
            .any(|digest| !valid_sha256(digest))
        {
            return Err(MigrationError::new(
                "migration-replacement-controls-incomplete",
            ));
        }
        let all_digests = [
            self.old_result_sha256.as_str(),
            self.new_result_sha256.as_str(),
            self.journey_result_sha256.as_str(),
            self.rollback_result_sha256.as_str(),
        ]
        .into_iter()
        .chain(
            self.false_pass_control_results
                .values()
                .map(|(_, digest)| digest.as_str()),
        )
        .collect::<Vec<_>>();
        if all_digests.iter().copied().collect::<BTreeSet<_>>().len() == 1 {
            return Err(MigrationError::new(
                "migration-replacement-repeated-digest-refused",
            ));
        }
        if self
            .false_pass_control_results
            .values()
            .map(|(_, digest)| digest)
            .collect::<BTreeSet<_>>()
            .len()
            != REQUIRED_FALSE_PASS_CONTROLS.len()
        {
            return Err(MigrationError::new(
                "migration-replacement-controls-incomplete",
            ));
        }
        Ok(())
    }

    fn digest_fragment(&self) -> String {
        let controls = self
            .false_pass_control_results
            .iter()
            .map(|(control, (verdict, digest))| format!("{control}:{verdict:?}:{digest}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}|{:?}|{}|{}|{:?}|{}|{}|{:?}|{}|{}|{}|{:?}|{}",
            self.old_behavior_id,
            self.old_verdict,
            self.old_result_sha256,
            self.new_behavior_id,
            self.new_verdict,
            self.new_result_sha256,
            self.journey_execution_id,
            self.journey_verdict,
            self.journey_result_sha256,
            controls,
            self.rollback_execution_id,
            self.rollback_verdict,
            self.rollback_result_sha256,
        )
    }
}

/// Opaque, root-observed replacement evidence. There is no public constructor,
/// clone, or deserialize path.
#[derive(Eq, PartialEq)]
pub struct ReplacementEvidence {
    source_id: String,
    canonical_target_id: String,
    route_id: String,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    observation: ReplacementObservation,
    semantic_sha256: String,
    reviewer_id: String,
    authority_id: String,
    authority_session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    binding_sha256: String,
    evidence_id: String,
    attestation_sha256: String,
}

impl fmt::Debug for ReplacementEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplacementEvidence")
            .field("contents", &"<redacted>")
            .finish()
    }
}
