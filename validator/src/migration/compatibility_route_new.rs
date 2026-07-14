pub struct CompatibilityRouteDefinition {
    pub route_id: String,
    pub source_id: String,
    pub canonical_target_id: String,
    pub owner_id: String,
    pub warning: String,
    pub usage_measurement_sha256: String,
    pub compatibility_boundary: String,
    pub removal_condition: String,
    pub equivalence_sha256: String,
    pub observed_invocations: u64,
    pub compatibility_window_complete: bool,
}

impl CompatibilityRoute {
    pub fn new(definition: CompatibilityRouteDefinition) -> Self {
        let CompatibilityRouteDefinition {
            route_id,
            source_id,
            canonical_target_id,
            owner_id,
            warning,
            usage_measurement_sha256,
            compatibility_boundary,
            removal_condition,
            equivalence_sha256,
            observed_invocations,
            compatibility_window_complete,
        } = definition;
        Self {
            route_id,
            source_id,
            canonical_target_id,
            owner_id,
            warning,
            usage_measurement_sha256,
            compatibility_boundary,
            removal_condition,
            equivalence_sha256,
            observed_invocations,
            compatibility_window_complete,
        }
    }

    pub fn route_id(&self) -> &str {
        &self.route_id
    }

    pub(crate) fn source_id(&self) -> &str {
        &self.source_id
    }

    pub(crate) fn canonical_target_id(&self) -> &str {
        &self.canonical_target_id
    }

    pub fn validate(&self) -> Result<(), MigrationError> {
        for value in [
            self.route_id.as_str(),
            self.source_id.as_str(),
            self.canonical_target_id.as_str(),
            self.owner_id.as_str(),
        ] {
            if !valid_identifier(value) {
                return Err(MigrationError::new("migration-route-identity-invalid"));
            }
        }
        if self.source_id == self.canonical_target_id {
            return Err(MigrationError::new("migration-route-self-target"));
        }
        if self.warning.is_empty()
            || self.warning.len() > 512
            || self.warning.chars().any(char::is_control)
            || !self.warning.to_ascii_lowercase().contains("compatib")
        {
            return Err(MigrationError::new("migration-route-warning-invalid"));
        }
        if !valid_sha256(&self.usage_measurement_sha256) || !valid_sha256(&self.equivalence_sha256)
        {
            return Err(MigrationError::new("migration-route-proof-invalid"));
        }
        if !safe_reference(&self.compatibility_boundary) || !safe_reference(&self.removal_condition)
        {
            return Err(MigrationError::new("migration-route-boundary-invalid"));
        }
        Ok(())
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.route_id,
            self.source_id,
            self.canonical_target_id,
            self.owner_id,
            self.warning,
            self.usage_measurement_sha256,
            self.compatibility_boundary,
            self.removal_condition,
            self.equivalence_sha256,
            self.observed_invocations,
            self.compatibility_window_complete
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceVerdict {
    Passed,
    CausalFailure,
}

/// Kernel-issued key for one persistent replacement-evidence ledger row. The
/// constructor is private so adapters can verify or store a key but cannot
/// reconstruct one from caller-supplied embedded plan fields.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ReplacementLedgerBinding {
    route_id: String,
    source_id: String,
    canonical_target_id: String,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    plan_sha256: String,
    target_id: String,
    target_sha256: String,
    reviewer_id: String,
    authority_id: String,
    authority_session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    evidence_binding_sha256: String,
    evidence_id: String,
    attestation_sha256: String,
    consumption_sha256: String,
    consumption_binding_sha256: String,
    ledger_binding_sha256: String,
}

impl ReplacementLedgerBinding {
    pub(crate) fn route_id(&self) -> &str {
        &self.route_id
    }

    pub(crate) fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    pub(crate) fn ledger_binding_sha256(&self) -> &str {
        &self.ledger_binding_sha256
    }

    pub(crate) fn authority_id(&self) -> &str {
        &self.authority_id
    }

    pub(crate) fn authority_session_id(&self) -> &str {
        &self.authority_session_id
    }

    pub(crate) fn consumption_sha256(&self) -> &str {
        &self.consumption_sha256
    }
}

impl fmt::Debug for ReplacementLedgerBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplacementLedgerBinding")
            .field("contents", &"<redacted>")
            .finish()
    }
}
