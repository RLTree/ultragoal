impl ApplyAuthorizationAuthority for DarwinMigrationAuthority {
    fn principal_id(&self) -> &str {
        &self.context.config().principal_id
    }

    fn authority_id(&self) -> &str {
        &self.context.config().authorization_authority_id
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn nonce_sha256(&self) -> &str {
        &self.nonce_sha256
    }

    fn issued_at_unix_ms(&self) -> u64 {
        self.issued_at_unix_ms
    }

    fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at_unix_ms
    }

    fn now_unix_ms(&self) -> u64 {
        self.context
            .trusted_time()
            .map(|time| time.unix_ms)
            .unwrap_or(u64::MAX)
    }

    fn current_binding(&self) -> (&str, &str) {
        (&self.input_binding_sha256, &self.plan_sha256)
    }

    fn seal(&mut self, binding_sha256: &str) -> Result<String, ProductMigrationError> {
        self.seal_value(AUTHORIZATION_SEAL_DOMAIN, binding_sha256)
            .map_err(HostError::product)
    }

    fn verify_seal(&self, binding_sha256: &str, seal_sha256: &str) -> bool {
        self.verify_value(AUTHORIZATION_SEAL_DOMAIN, binding_sha256, seal_sha256)
    }

    fn compatibility_boundary_authority_id(&self) -> &str {
        &self.context.config().compatibility_boundary_authority_id
    }

    fn compatibility_boundary_source_identity_sha256(&self) -> &str {
        &self
            .context
            .config()
            .compatibility_boundary_source_identity_sha256
    }

    fn compatibility_boundary_observation_sequence(&self) -> u64 {
        self.current_boundary_time(true).monotonic_ns
    }

    fn compatibility_boundary_observed_at_unix_ms(&self) -> u64 {
        self.current_boundary_time(false).unix_ms
    }

    fn compatibility_boundary_current_product_version(&self) -> &str {
        &self.context.config().product_version
    }

    fn seal_compatibility_boundary(
        &self,
        binding_sha256: &str,
    ) -> Result<String, ProductMigrationError> {
        self.seal_value(BOUNDARY_SEAL_DOMAIN, binding_sha256)
            .map_err(HostError::product)
    }

    fn verify_compatibility_boundary_seal(&self, binding_sha256: &str, seal_sha256: &str) -> bool {
        let valid = self.verify_value(BOUNDARY_SEAL_DOMAIN, binding_sha256, seal_sha256);
        if valid {
            self.mark_boundary_observed(binding_sha256);
        }
        valid
    }
}

fn random_digest(domain: &str) -> Result<String, HostError> {
    let mut random = [0_u8; 32];
    getrandom::fill(&mut random)
        .map_err(|_| HostError::new("migration-host-random-unavailable"))?;
    let mut bytes = domain.as_bytes().to_vec();
    bytes.push(b'|');
    bytes.extend_from_slice(&random);
    Ok(digest_bytes(&bytes))
}

fn boundary_binding(
    config: &super::HostAuthorityConfig,
    sequence: u64,
    observed_at_unix_ms: u64,
) -> String {
    digest_bytes(
        format!(
            "migration-compatibility-boundary-observation-binding-v1|{}|{}|{}|{}|{}",
            config.compatibility_boundary_authority_id,
            config.compatibility_boundary_source_identity_sha256,
            sequence,
            observed_at_unix_ms,
            config.product_version,
        )
        .as_bytes(),
    )
}
