impl DarwinMigrationAuthority {
    fn issue(
        context: Arc<HostContext>,
        input_binding_sha256: String,
        plan_sha256: String,
        forbidden_session: Option<&str>,
    ) -> Result<Self, HostError> {
        context.verify_static()?;
        if !super::super::super::valid_sha256(&input_binding_sha256)
            || !super::super::super::valid_sha256(&plan_sha256)
        {
            return Err(HostError::new("migration-host-authority-binding-invalid"));
        }
        let time = context.trusted_time()?;
        let session_id = random_digest(SESSION_DOMAIN)?;
        if forbidden_session == Some(session_id.as_str()) {
            return Err(HostError::new("migration-host-authority-session-collision"));
        }
        let nonce_sha256 = random_digest(NONCE_DOMAIN)?;
        let expires_at_unix_ms = time
            .unix_ms
            .checked_add(AUTHORIZATION_TTL_MS)
            .ok_or_else(|| HostError::new("migration-host-clock-invalid"))?;
        Ok(Self {
            context,
            session_id,
            nonce_sha256,
            issued_at_unix_ms: time.unix_ms,
            expires_at_unix_ms,
            input_binding_sha256,
            plan_sha256,
            boundary: Mutex::new(BoundaryState {
                time,
                refresh_on_next_sequence: false,
            }),
        })
    }

    fn recover(
        context: Arc<HostContext>,
        input_binding_sha256: String,
        plan_sha256: String,
        durable: DurableRecoveryAuthorization,
    ) -> Result<Self, HostError> {
        context.verify_static()?;
        let time = context.trusted_time()?;
        if durable.issued_at_unix_ms == 0
            || durable.expires_at_unix_ms <= durable.issued_at_unix_ms
            || durable.expires_at_unix_ms - durable.issued_at_unix_ms > 10 * 60 * 1_000
        {
            return Err(HostError::new(
                "migration-host-recovery-authority-window-refused",
            ));
        }
        Ok(Self {
            context,
            session_id: durable.authority_session_id,
            nonce_sha256: durable.nonce_sha256,
            issued_at_unix_ms: durable.issued_at_unix_ms,
            expires_at_unix_ms: durable.expires_at_unix_ms,
            input_binding_sha256,
            plan_sha256,
            boundary: Mutex::new(BoundaryState {
                time,
                refresh_on_next_sequence: false,
            }),
        })
    }

    fn current_boundary_time(&self, refresh: bool) -> TrustedTime {
        let mut boundary = self
            .boundary
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if refresh && boundary.refresh_on_next_sequence {
            boundary.time = self.context.trusted_time().unwrap_or(TrustedTime {
                monotonic_ns: u64::MAX,
                unix_ms: u64::MAX,
            });
            boundary.refresh_on_next_sequence = false;
        }
        boundary.time
    }

    fn mark_boundary_observed(&self, binding_sha256: &str) {
        let mut boundary = self
            .boundary
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if boundary_binding(
            self.context.config(),
            boundary.time.monotonic_ns,
            boundary.time.unix_ms,
        ) == binding_sha256
        {
            boundary.refresh_on_next_sequence = true;
        }
    }

    fn seal_value(&self, domain: &str, binding_sha256: &str) -> Result<String, HostError> {
        self.context.verify_static()?;
        if !super::super::super::valid_sha256(binding_sha256) {
            return Err(HostError::new("migration-host-seal-binding-invalid"));
        }
        let mut mac = Hmac::<Sha256>::new_from_slice(self.context.key())
            .map_err(|_| HostError::new("migration-host-secret-key-invalid"))?;
        mac.update(domain.as_bytes());
        mac.update(b"|");
        mac.update(binding_sha256.as_bytes());
        Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
    }

    fn verify_value(&self, domain: &str, binding_sha256: &str, seal_sha256: &str) -> bool {
        self.seal_value(domain, binding_sha256)
            .map(|expected| expected == seal_sha256)
            .unwrap_or(false)
    }
}
