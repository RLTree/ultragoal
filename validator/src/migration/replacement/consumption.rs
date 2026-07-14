impl ReplacementEvidence {
    fn consume<A: ReplacementEvidenceAuthority>(
        self,
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        authority: &mut A,
    ) -> Result<ConsumedReplacementEvidence, MigrationError> {
        self.validate_current(inventory, route, authority)?;
        let consumption_sha256 = authority.consume_once(
            &self.route_id,
            &self.binding_sha256,
            &self.evidence_id,
            &self.attestation_sha256,
        )?;
        if !valid_sha256(&consumption_sha256)
            || !authority.verify_consumed(
                &self.route_id,
                &self.binding_sha256,
                &self.evidence_id,
                &self.attestation_sha256,
                &consumption_sha256,
            )
            || self.validate_current(inventory, route, authority).is_err()
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-consumption-refused",
            ));
        }
        let consumption_binding_sha256 = replacement_consumption_binding(
            &self.route_id,
            &self.binding_sha256,
            &self.evidence_id,
            &self.attestation_sha256,
            &consumption_sha256,
        );
        Ok(ConsumedReplacementEvidence {
            source_id: self.source_id,
            canonical_target_id: self.canonical_target_id,
            route_id: self.route_id,
            live_context_id: self.live_context_id,
            candidate_id: self.candidate_id,
            catalog_id: self.catalog_id,
            read_session_id: self.read_session_id,
            inventory_sha256: self.inventory_sha256,
            observation: self.observation,
            semantic_sha256: self.semantic_sha256,
            reviewer_id: self.reviewer_id,
            authority_id: self.authority_id,
            authority_session_id: self.authority_session_id,
            nonce_sha256: self.nonce_sha256,
            issued_at_unix_ms: self.issued_at_unix_ms,
            expires_at_unix_ms: self.expires_at_unix_ms,
            binding_sha256: self.binding_sha256,
            evidence_id: self.evidence_id,
            attestation_sha256: self.attestation_sha256,
            consumption_sha256,
            consumption_binding_sha256,
        })
    }

    #[cfg(test)]
    pub(crate) fn substitute_semantic_for_test(&mut self, digest: impl Into<String>) {
        self.semantic_sha256 = digest.into();
    }

    #[cfg(test)]
    pub(crate) fn substitute_attestation_for_test(&mut self, digest: impl Into<String>) {
        self.attestation_sha256 = digest.into();
    }
}

struct ReplacementEvidenceBinding<'a> {
    inventory: &'a MigrationInventory,
    route: &'a CompatibilityRoute,
    observation: &'a ReplacementObservation,
    semantic_sha256: &'a str,
    reviewer_id: &'a str,
    authority_id: &'a str,
    authority_session_id: &'a str,
    nonce_sha256: &'a str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
}

fn replacement_evidence_binding(binding: ReplacementEvidenceBinding<'_>) -> String {
    let ReplacementEvidenceBinding {
        inventory,
        route,
        observation,
        semantic_sha256,
        reviewer_id,
        authority_id,
        authority_session_id,
        nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
    } = binding;
    digest(
        format!(
            "replacement-evidence-v2|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            inventory.live_context_id,
            inventory.candidate_id,
            inventory.catalog_id,
            inventory.read_session_id,
            inventory.inventory_sha256,
            route.route_id,
            route.source_id,
            route.canonical_target_id,
            route.digest_fragment(),
            observation.digest_fragment(),
            semantic_sha256,
            reviewer_id,
            authority_id,
            authority_session_id,
            nonce_sha256,
            format_args!("{issued_at_unix_ms}:{expires_at_unix_ms}"),
        )
        .as_bytes(),
    )
}

struct ReplacementAuthorityMatch<'a, A: ReplacementEvidenceAuthority + ?Sized> {
    binding: ReplacementEvidenceBinding<'a>,
    authority: &'a A,
}

fn replacement_authority_matches<A: ReplacementEvidenceAuthority + ?Sized>(
    request: ReplacementAuthorityMatch<'_, A>,
) -> bool {
    let ReplacementAuthorityMatch { binding, authority } = request;
    let ReplacementEvidenceBinding {
        inventory,
        route,
        observation,
        reviewer_id,
        authority_id,
        authority_session_id,
        nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
        ..
    } = binding;
    let current_binding = authority.current_binding();
    authority.authority_id() == authority_id
        && authority.reviewer_id(&route.route_id) == Some(reviewer_id)
        && authority.session_id() == authority_session_id
        && authority.nonce_sha256(&route.route_id) == Some(nonce_sha256)
        && authority.issued_at_unix_ms() == issued_at_unix_ms
        && authority.expires_at_unix_ms() == expires_at_unix_ms
        && valid_identifier(reviewer_id)
        && valid_identifier(authority_id)
        && reviewer_id != authority_id
        && reviewer_id != route.owner_id
        && authority_id != route.owner_id
        && valid_sha256(authority_session_id)
        && authority_session_id != inventory.read_session_id
        && valid_sha256(nonce_sha256)
        && valid_authority_window(
            issued_at_unix_ms,
            expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        && current_binding
            == (
                inventory.live_context_id.as_str(),
                inventory.candidate_id.as_str(),
                inventory.catalog_id.as_str(),
                inventory.read_session_id.as_str(),
                inventory.inventory_sha256.as_str(),
            )
        && ReplacementObservation::capture(route, authority).as_ref() == Ok(observation)
}

#[derive(Eq, PartialEq)]
struct ConsumedReplacementEvidence {
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
    consumption_sha256: String,
    consumption_binding_sha256: String,
}

impl fmt::Debug for ConsumedReplacementEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConsumedReplacementEvidence")
            .field("contents", &"<redacted>")
            .finish()
    }
}
