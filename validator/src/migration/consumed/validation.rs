impl ConsumedReplacementEvidence {
    fn validate(
        &self,
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        now_unix_ms: u64,
    ) -> Result<(), MigrationError> {
        self.observation.validate(route)?;
        let semantic_sha256 = digest(self.observation.digest_fragment().as_bytes());
        let binding_sha256 = replacement_evidence_binding(ReplacementEvidenceBinding {
            inventory,
            route,
            observation: &self.observation,
            semantic_sha256: &semantic_sha256,
            reviewer_id: &self.reviewer_id,
            authority_id: &self.authority_id,
            authority_session_id: &self.authority_session_id,
            nonce_sha256: &self.nonce_sha256,
            issued_at_unix_ms: self.issued_at_unix_ms,
            expires_at_unix_ms: self.expires_at_unix_ms,
        });
        let evidence_id = digest(
            format!(
                "replacement-evidence|{}|{}",
                binding_sha256, self.attestation_sha256
            )
            .as_bytes(),
        );
        let consumption_binding_sha256 = replacement_consumption_binding(
            &self.route_id,
            &binding_sha256,
            &evidence_id,
            &self.attestation_sha256,
            &self.consumption_sha256,
        );
        if self.source_id != route.source_id
            || self.canonical_target_id != route.canonical_target_id
            || self.route_id != route.route_id
            || self.live_context_id != inventory.live_context_id
            || self.candidate_id != inventory.candidate_id
            || self.catalog_id != inventory.catalog_id
            || self.read_session_id != inventory.read_session_id
            || self.inventory_sha256 != inventory.inventory_sha256
            || self.semantic_sha256 != semantic_sha256
            || self.binding_sha256 != binding_sha256
            || self.evidence_id != evidence_id
            || !valid_identifier(&self.reviewer_id)
            || !valid_identifier(&self.authority_id)
            || self.reviewer_id == self.authority_id
            || self.reviewer_id == route.owner_id
            || self.authority_id == route.owner_id
            || !valid_sha256(&self.authority_session_id)
            || self.authority_session_id == inventory.read_session_id
            || !valid_sha256(&self.nonce_sha256)
            || !valid_authority_window(self.issued_at_unix_ms, self.expires_at_unix_ms, now_unix_ms)
            || !valid_sha256(&self.attestation_sha256)
            || !valid_sha256(&self.consumption_sha256)
            || self.consumption_binding_sha256 != consumption_binding_sha256
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-invalid",
            ));
        }
        Ok(())
    }

    fn validate_with_authority<A: ReplacementEvidenceAuthority + ?Sized>(
        &self,
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        authority: &A,
    ) -> Result<(), MigrationError> {
        self.validate(inventory, route, authority.now_unix_ms())?;
        let semantic_sha256 = digest(self.observation.digest_fragment().as_bytes());
        if !replacement_authority_matches(ReplacementAuthorityMatch {
            binding: ReplacementEvidenceBinding {
                inventory,
                route,
                observation: &self.observation,
                semantic_sha256: &semantic_sha256,
                reviewer_id: &self.reviewer_id,
                authority_id: &self.authority_id,
                authority_session_id: &self.authority_session_id,
                nonce_sha256: &self.nonce_sha256,
                issued_at_unix_ms: self.issued_at_unix_ms,
                expires_at_unix_ms: self.expires_at_unix_ms,
            },
            authority,
        }) || !authority.verify_attestation(
            &self.route_id,
            &self.binding_sha256,
            &self.attestation_sha256,
        ) || !authority.verify_consumed(
            &self.route_id,
            &self.binding_sha256,
            &self.evidence_id,
            &self.attestation_sha256,
            &self.consumption_sha256,
        ) {
            return Err(MigrationError::new(
                "migration-replacement-ledger-not-current",
            ));
        }
        Ok(())
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.source_id,
            self.canonical_target_id,
            self.route_id,
            self.live_context_id,
            self.candidate_id,
            self.catalog_id,
            self.read_session_id,
            self.inventory_sha256,
            self.observation.digest_fragment(),
            self.semantic_sha256,
            self.reviewer_id,
            self.authority_id,
            self.authority_session_id,
            self.nonce_sha256,
            self.issued_at_unix_ms,
            self.expires_at_unix_ms,
            self.binding_sha256,
            self.evidence_id,
            self.attestation_sha256,
            self.consumption_sha256,
            self.consumption_binding_sha256,
        )
    }
}
