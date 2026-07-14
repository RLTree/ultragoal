impl ReplacementEvidence {
    pub(crate) fn issue<A: ReplacementEvidenceAuthority>(
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        authority: &mut A,
    ) -> Result<Self, MigrationError> {
        inventory.validate()?;
        route.validate()?;
        let observation = ReplacementObservation::capture(route, authority)?;
        let reviewer_id = authority
            .reviewer_id(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-reviewer-missing"))?
            .to_owned();
        let authority_id = authority.authority_id().to_owned();
        let authority_session_id = authority.session_id().to_owned();
        let nonce_sha256 = authority
            .nonce_sha256(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-nonce-missing"))?
            .to_owned();
        let issued_at_unix_ms = authority.issued_at_unix_ms();
        let expires_at_unix_ms = authority.expires_at_unix_ms();
        let (context, candidate, catalog, read_session, inventory_sha256) =
            authority.current_binding();
        if !valid_identifier(&reviewer_id)
            || !valid_identifier(&authority_id)
            || reviewer_id == authority_id
            || reviewer_id == route.owner_id
            || authority_id == route.owner_id
            || !valid_sha256(&authority_session_id)
            || authority_session_id == inventory.read_session_id
            || !valid_sha256(&nonce_sha256)
            || !valid_authority_window(
                issued_at_unix_ms,
                expires_at_unix_ms,
                authority.now_unix_ms(),
            )
            || (context, candidate, catalog, read_session, inventory_sha256)
                != (
                    inventory.live_context_id.as_str(),
                    inventory.candidate_id.as_str(),
                    inventory.catalog_id.as_str(),
                    inventory.read_session_id.as_str(),
                    inventory.inventory_sha256.as_str(),
                )
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-issuance-refused",
            ));
        }
        let semantic_sha256 = digest(observation.digest_fragment().as_bytes());
        let binding_sha256 = replacement_evidence_binding(ReplacementEvidenceBinding {
            inventory,
            route,
            observation: &observation,
            semantic_sha256: &semantic_sha256,
            reviewer_id: &reviewer_id,
            authority_id: &authority_id,
            authority_session_id: &authority_session_id,
            nonce_sha256: &nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
        });
        let attestation_sha256 = authority.issue_attestation(&route.route_id, &binding_sha256)?;
        let evidence_id = digest(
            format!("replacement-evidence|{binding_sha256}|{attestation_sha256}").as_bytes(),
        );
        if !valid_sha256(&attestation_sha256)
            || !authority.verify_attestation(&route.route_id, &binding_sha256, &attestation_sha256)
            || !replacement_authority_matches(ReplacementAuthorityMatch {
                binding: ReplacementEvidenceBinding {
                    inventory,
                    route,
                    observation: &observation,
                    semantic_sha256: &semantic_sha256,
                    reviewer_id: &reviewer_id,
                    authority_id: &authority_id,
                    authority_session_id: &authority_session_id,
                    nonce_sha256: &nonce_sha256,
                    issued_at_unix_ms,
                    expires_at_unix_ms,
                },
                authority,
            })
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-issuance-refused",
            ));
        }
        Ok(Self {
            source_id: route.source_id.clone(),
            canonical_target_id: route.canonical_target_id.clone(),
            route_id: route.route_id.clone(),
            live_context_id: inventory.live_context_id.clone(),
            candidate_id: inventory.candidate_id.clone(),
            catalog_id: inventory.catalog_id.clone(),
            read_session_id: inventory.read_session_id.clone(),
            inventory_sha256: inventory.inventory_sha256.clone(),
            observation,
            semantic_sha256,
            reviewer_id,
            authority_id,
            authority_session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            binding_sha256,
            evidence_id,
            attestation_sha256,
        })
    }

    fn validate_current<A: ReplacementEvidenceAuthority>(
        &self,
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        authority: &A,
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
            || !valid_sha256(&self.attestation_sha256)
            || !authority.verify_attestation(
                &route.route_id,
                &binding_sha256,
                &self.attestation_sha256,
            )
            || !replacement_authority_matches(ReplacementAuthorityMatch {
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
            })
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-stale-or-substituted",
            ));
        }
        Ok(())
    }
}
