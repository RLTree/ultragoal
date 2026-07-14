impl HostContext {
    fn open(
        repository_root: &Path,
        state_root: &Path,
        expected_candidate_id: &str,
        expected_product_version: &str,
    ) -> Result<Arc<Self>, HostError> {
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (
                repository_root,
                state_root,
                expected_candidate_id,
                expected_product_version,
            );
            return Err(HostError::new("migration-host-darwin-required"));
        }
        #[cfg(target_vendor = "apple")]
        {
            let repository = AnchoredDirectory::open_absolute(repository_root, false)?;
            let state = AnchoredDirectory::open_absolute(state_root, true)?;
            if state.path().starts_with(repository.path())
                || repository.path().starts_with(state.path())
            {
                return Err(HostError::new("migration-host-root-overlap-refused"));
            }
            let expected_scope = repository_scope_id(
                repository.path(),
                repository.identity(),
                expected_candidate_id,
            );
            let config_file = state.read_regular(CONFIG_NAME, true, MAX_CONFIG_BYTES)?;
            let config: HostAuthorityConfig = serde_json::from_slice(&config_file.bytes)
                .map_err(|_| HostError::new("migration-host-authority-config-invalid"))?;
            config.validate()?;
            if config.canonical_bytes()? != config_file.bytes
                || config.repository_scope_sha256 != expected_scope
                || config.candidate_id != expected_candidate_id
                || config.product_version != expected_product_version
            {
                return Err(HostError::new("migration-host-authority-binding-refused"));
            }
            let key_file = state.read_regular(KEY_NAME, true, MAX_KEY_BYTES)?;
            if key_file.bytes.len() != KEY_BYTES {
                return Err(HostError::new("migration-host-secret-key-invalid"));
            }
            let mut key = [0_u8; KEY_BYTES];
            key.copy_from_slice(&key_file.bytes);
            let lock_identity = state.fixed_file_identity(LOCK_NAME, 0)?;
            if lock_identity.size != 0 {
                return Err(HostError::new("migration-host-lock-file-refused"));
            }
            let _ = state.fixed_file_identity(LEDGER_NAME, MAX_HOST_FILE_BYTES)?;
            let process_lock = ProcessLock::acquire(&state, LOCK_NAME)?;
            let value = Arc::new(Self {
                repository,
                state,
                config,
                config_bytes: config_file.bytes,
                config_identity: config_file.identity,
                key,
                key_bytes: key_file.bytes,
                key_identity: key_file.identity,
                lock_identity,
                _process_lock: process_lock,
                io: Mutex::new(()),
                #[cfg(test)]
                trusted_time_advance_ms: std::sync::atomic::AtomicU64::new(0),
                #[cfg(test)]
                fail_on_cas: Mutex::new(None),
                #[cfg(test)]
                cas_count: std::sync::atomic::AtomicUsize::new(0),
            });
            value.verify_static()?;
            let _ = value.trusted_time()?;
            Ok(value)
        }
    }

    pub(super) fn verify_static(&self) -> Result<(), HostError> {
        self.repository.verify()?;
        self.state.verify()?;
        self.state.verify_fixed_file(
            CONFIG_NAME,
            self.config_identity,
            &self.config_bytes,
            MAX_CONFIG_BYTES,
        )?;
        self.state.verify_fixed_file(
            KEY_NAME,
            self.key_identity,
            &self.key_bytes,
            MAX_KEY_BYTES,
        )?;
        if self.state.fixed_file_identity(LOCK_NAME, 0)? != self.lock_identity {
            return Err(HostError::new("migration-host-lock-substituted"));
        }
        self._process_lock.verify(&self.state, LOCK_NAME)?;
        let ledger = self
            .state
            .fixed_file_identity(LEDGER_NAME, MAX_HOST_FILE_BYTES)?;
        if ledger.size == 0 {
            return Err(HostError::new("migration-host-state-empty"));
        }
        Ok(())
    }

    pub(super) fn trusted_time(&self) -> Result<TrustedTime, HostError> {
        self.verify_static_without_clock()?;
        let monotonic_ns = monotonic_nanoseconds()?;
        #[cfg(test)]
        let monotonic_ns = self
            .trusted_time_advance_ms
            .load(std::sync::atomic::Ordering::SeqCst)
            .checked_mul(1_000_000)
            .and_then(|advance| monotonic_ns.checked_add(advance))
            .ok_or_else(|| HostError::new("migration-host-clock-invalid"))?;
        if monotonic_ns < self.config.clock_anchor_monotonic_ns {
            return Err(HostError::new("migration-host-clock-rollback"));
        }
        let delta_ms = monotonic_ns
            .checked_sub(self.config.clock_anchor_monotonic_ns)
            .ok_or_else(|| HostError::new("migration-host-clock-rollback"))?
            / 1_000_000;
        let unix_ms = self
            .config
            .clock_anchor_unix_ms
            .checked_add(delta_ms)
            .ok_or_else(|| HostError::new("migration-host-clock-invalid"))?;
        Ok(TrustedTime {
            monotonic_ns,
            unix_ms,
        })
    }

    fn verify_static_without_clock(&self) -> Result<(), HostError> {
        self.repository.verify()?;
        self.state.verify()?;
        self.state.verify_fixed_file(
            CONFIG_NAME,
            self.config_identity,
            &self.config_bytes,
            MAX_CONFIG_BYTES,
        )?;
        self.state.verify_fixed_file(
            KEY_NAME,
            self.key_identity,
            &self.key_bytes,
            MAX_KEY_BYTES,
        )?;
        self._process_lock.verify(&self.state, LOCK_NAME)
    }

    fn state(&self) -> &AnchoredDirectory {
        &self.state
    }

    fn repository(&self) -> &AnchoredDirectory {
        &self.repository
    }

    pub(super) fn scope_id(&self) -> &str {
        &self.config.repository_scope_sha256
    }

    pub(super) fn config(&self) -> &HostAuthorityConfig {
        &self.config
    }

    pub(super) fn key(&self) -> &[u8; KEY_BYTES] {
        &self.key
    }

    pub(super) fn io(&self) -> &Mutex<()> {
        &self.io
    }

    #[cfg(test)]
    pub(super) fn advance_time_for_test(&self, milliseconds: u64) {
        self.trusted_time_advance_ms
            .fetch_add(milliseconds, std::sync::atomic::Ordering::SeqCst);
    }
}
