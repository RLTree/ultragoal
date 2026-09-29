impl CompatibilityBoundaryObservation {
    #[cfg(test)]
    fn issue(
        authority_id: String,
        source_identity_sha256: String,
        observation_sequence: u64,
        observed_at_unix_ms: u64,
        current_product_version: String,
        binding_sha256: String,
        seal_sha256: String,
    ) -> Result<Self, ProductMigrationError> {
        let observation_sha256 = digest(
            format!(
                "migration-compatibility-boundary-observation-v1|{}|{}",
                binding_sha256, seal_sha256
            )
            .as_bytes(),
        );
        let value = Self {
            schema_version: "CompatibilityBoundaryObservation-v1".to_owned(),
            authority_id,
            source_identity_sha256,
            observation_sequence,
            observed_at_unix_ms,
            current_product_version,
            binding_sha256,
            seal_sha256,
            observation_sha256,
        };
        if !value.validate() {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-observation-invalid",
            ));
        }
        Ok(value)
    }

    #[cfg(test)]
    pub(super) fn observation_sha256(&self) -> &str {
        &self.observation_sha256
    }

    pub(super) fn validate(&self) -> bool {
        self.schema_version == "CompatibilityBoundaryObservation-v1"
            && valid_identifier(&self.authority_id)
            && valid_sha256(&self.source_identity_sha256)
            && parse_product_version(&self.current_product_version).is_some()
            && self.binding_sha256
                == compatibility_boundary_observation_binding(
                    &self.authority_id,
                    &self.source_identity_sha256,
                    self.observation_sequence,
                    self.observed_at_unix_ms,
                    &self.current_product_version,
                )
            && valid_sha256(&self.seal_sha256)
            && self.observation_sha256
                == digest(
                    format!(
                        "migration-compatibility-boundary-observation-v1|{}|{}",
                        self.binding_sha256, self.seal_sha256
                    )
                    .as_bytes(),
                )
    }

    #[cfg(test)]
    pub(super) fn is_same_source_and_monotonic_after(&self, prior: &Self) -> bool {
        if !self.validate()
            || !prior.validate()
            || self.authority_id != prior.authority_id
            || self.source_identity_sha256 != prior.source_identity_sha256
            || self.observation_sequence < prior.observation_sequence
            || self.observed_at_unix_ms < prior.observed_at_unix_ms
        {
            return false;
        }
        let Some(current) = parse_product_version(&self.current_product_version) else {
            return false;
        };
        let Some(previous) = parse_product_version(&prior.current_product_version) else {
            return false;
        };
        if current < previous {
            return false;
        }
        let state_changed = self.observed_at_unix_ms != prior.observed_at_unix_ms
            || self.current_product_version != prior.current_product_version;
        !state_changed || self.observation_sequence > prior.observation_sequence
    }

    #[cfg(test)]
    pub(super) fn seal_verified_by(&self, authority: &dyn ApplyAuthorizationAuthority) -> bool {
        self.validate()
            && authority.compatibility_boundary_authority_id() == self.authority_id
            && authority.compatibility_boundary_source_identity_sha256()
                == self.source_identity_sha256
            && authority.verify_compatibility_boundary_seal(&self.binding_sha256, &self.seal_sha256)
    }
}

#[cfg(test)]
pub(super) fn capture_compatibility_boundary_observation(
    authority: &dyn ApplyAuthorizationAuthority,
) -> Result<CompatibilityBoundaryObservation, ProductMigrationError> {
    let authority_id = authority.compatibility_boundary_authority_id().to_owned();
    let source_identity_sha256 = authority
        .compatibility_boundary_source_identity_sha256()
        .to_owned();
    let observation_sequence = authority.compatibility_boundary_observation_sequence();
    let observed_at_unix_ms = authority.compatibility_boundary_observed_at_unix_ms();
    let current_product_version = authority
        .compatibility_boundary_current_product_version()
        .to_owned();
    let binding_sha256 = compatibility_boundary_observation_binding(
        &authority_id,
        &source_identity_sha256,
        observation_sequence,
        observed_at_unix_ms,
        &current_product_version,
    );
    let seal_sha256 = authority.seal_compatibility_boundary(&binding_sha256)?;
    if authority.compatibility_boundary_authority_id() != authority_id
        || authority.compatibility_boundary_source_identity_sha256() != source_identity_sha256
        || authority.compatibility_boundary_observation_sequence() != observation_sequence
        || authority.compatibility_boundary_observed_at_unix_ms() != observed_at_unix_ms
        || authority.compatibility_boundary_current_product_version() != current_product_version
        || !authority.verify_compatibility_boundary_seal(&binding_sha256, &seal_sha256)
    {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-authority-refused",
        ));
    }
    CompatibilityBoundaryObservation::issue(
        authority_id,
        source_identity_sha256,
        observation_sequence,
        observed_at_unix_ms,
        current_product_version,
        binding_sha256,
        seal_sha256,
    )
}

fn compatibility_boundary_observation_binding(
    authority_id: &str,
    source_identity_sha256: &str,
    observation_sequence: u64,
    observed_at_unix_ms: u64,
    current_product_version: &str,
) -> String {
    digest(
        format!(
            "migration-compatibility-boundary-observation-binding-v1|{}|{}|{}|{}|{}",
            authority_id,
            source_identity_sha256,
            observation_sequence,
            observed_at_unix_ms,
            current_product_version,
        )
        .as_bytes(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompatibilityUsageMeasurement {
    schema_version: String,
    route_id: String,
    metric: String,
    evidence_sha256: String,
    window_start_unix_ms: u64,
    window_end_unix_ms: u64,
    observed_invocations: u64,
}

impl CompatibilityUsageMeasurement {
    fn validate(&self, route_id: &str) -> bool {
        self.schema_version == "CompatibilityUsageMeasurement-v1"
            && self.route_id == route_id
            && valid_identifier(&self.route_id)
            && self.metric == "legacy-route-invocations"
            && valid_sha256(&self.evidence_sha256)
            && self.window_start_unix_ms < self.window_end_unix_ms
            && self
                .window_end_unix_ms
                .saturating_sub(self.window_start_unix_ms)
                <= MAX_COMPATIBILITY_MEASUREMENT_WINDOW_MS
            && self.observed_invocations <= MAX_COMPATIBILITY_OBSERVED_INVOCATIONS
    }
}
