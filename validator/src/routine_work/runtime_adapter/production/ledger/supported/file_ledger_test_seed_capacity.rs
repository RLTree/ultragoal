use super::*;

impl FileLedger {
    #[cfg(test)]
    pub(crate) fn test_seed_capacity(
        &self,
        protocol_effect_count: usize,
        consumed_grant_count: usize,
    ) -> Result<(), RoutineError> {
        if protocol_effect_count > MAX_RECORDS
            || consumed_grant_count > MAX_RECORDS.saturating_mul(4)
        {
            return Err(error("routine-production-test-capacity-invalid"));
        }
        self.with_payload(true, |payload, tick| {
            if payload.protocols.len() != payload.effects.len()
                || payload.protocols.len() > protocol_effect_count
                || payload.consumed_grants.len() > consumed_grant_count
            {
                return Err(error("routine-production-test-capacity-invalid"));
            }
            let mut ordinal = 0_u64;
            while payload.protocols.len() < protocol_effect_count {
                let protocol_id = test_digest("protocol", ordinal);
                let effect_id = test_digest("effect", ordinal);
                ordinal = ordinal
                    .checked_add(1)
                    .ok_or_else(|| error("routine-production-test-capacity-invalid"))?;
                if payload.protocols.contains_key(&protocol_id)
                    || payload.effects.contains_key(&effect_id)
                {
                    continue;
                }
                let grant_id = test_digest("grant", ordinal);
                let record = ProtocolRecord {
                    binding: AuthorityBinding {
                        protocol_id: protocol_id.clone(),
                        effect_id: effect_id.clone(),
                        context_id: test_digest("context", ordinal),
                        candidate_id: test_digest("candidate", ordinal),
                        plan_id: test_digest("plan", ordinal),
                        snapshot_id: test_digest("snapshot", ordinal),
                    },
                    request_id: test_digest("request", ordinal),
                    grant_id: grant_id.clone(),
                    recovery_marker: test_digest("recovery", ordinal),
                    recovery_for: None,
                    state: AttemptState::Failed,
                    reuse_only: false,
                    issued_tick: tick,
                    expires_tick: tick,
                    recovery_deadline_tick: tick,
                    artifacts: BTreeMap::new(),
                };
                payload.effects.insert(effect_id, protocol_id.clone());
                payload.protocols.insert(protocol_id, record);
                payload.consumed_grants.insert(grant_id);
            }
            if payload.consumed_grants.len() > consumed_grant_count {
                return Err(error("routine-production-test-capacity-invalid"));
            }
            let mut ordinal = 0_u64;
            while payload.consumed_grants.len() < consumed_grant_count {
                payload
                    .consumed_grants
                    .insert(test_digest("consumed", ordinal));
                ordinal = ordinal
                    .checked_add(1)
                    .ok_or_else(|| error("routine-production-test-capacity-invalid"))?;
            }
            Ok(())
        })
    }
    pub(crate) fn with_payload<T>(
        &self,
        write: bool,
        operation: impl FnOnce(&mut Payload, u64) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        self.with_payload_conditional(|payload, tick| {
            operation(payload, tick).map(|value| (value, write))
        })
    }
    pub(crate) fn with_payload_conditional<T>(
        &self,
        operation: impl FnOnce(&mut Payload, u64) -> Result<(T, bool), RoutineError>,
    ) -> Result<T, RoutineError> {
        let guard = self.acquire_lock()?;
        self.store.verify_root()?;
        let key_file = self.store.open_existing(KEY_NAME, libc::O_RDONLY)?;
        if self.store.exact_identity(KEY_NAME, &key_file, 0o600)? != self.key_identity {
            return Err(error("routine-production-authority-key-replaced"));
        }
        let key = read_key(&key_file)?;
        if sha256(&key.0) != self.key_id {
            return Err(error("routine-production-authority-key-mutated"));
        }
        let bytes = self.store.read_state()?;
        let state_identity = self.store.state_identity()?;
        let mut payload = decode(
            &bytes,
            &key,
            &self.authority_id,
            &self.key_id,
            self.store.identity,
            self.lock_identity,
        )?;
        validate_payload(&payload)?;
        let head_sha256 = sha256(&bytes);
        let mut local = self
            .local
            .lock()
            .map_err(|_| error("routine-production-local-lock-poisoned"))?;
        if payload.generation < local.generation
            || (payload.generation == local.generation
                && (head_sha256 != local.head_sha256 || state_identity != local.state_identity))
        {
            return Err(error("routine-production-authority-rollback-detected"));
        }
        let tick = now_tick()?;
        if tick < payload.last_tick {
            return Err(error("routine-production-trusted-time-regressed"));
        }
        let (value, write) = operation(&mut payload, tick)?;
        if write {
            payload.generation = payload
                .generation
                .checked_add(1)
                .ok_or_else(|| error("routine-production-generation-exhausted"))?;
            payload.previous_head_sha256 = head_sha256;
            payload.last_tick = tick;
            // Every write closure operates on an in-memory projection. Do
            // not publish bytes that a subsequent open would reject.
            validate_payload(&payload)?;
            let next = encode(&payload, &key)?;
            self.store.write_atomic_state(&next)?;
            let confirmed = self.store.read_state()?;
            if confirmed != next {
                return Err(error("routine-production-authority-publish-mismatch"));
            }
            let decoded = decode(
                &confirmed,
                &key,
                &self.authority_id,
                &self.key_id,
                self.store.identity,
                self.lock_identity,
            )?;
            if decoded != payload {
                return Err(error("routine-production-authority-revalidation-failed"));
            }
            local.generation = payload.generation;
            local.head_sha256 = sha256(&confirmed);
            local.state_identity = self.store.state_identity()?;
        } else {
            local.generation = payload.generation;
            local.head_sha256 = head_sha256;
            local.state_identity = state_identity;
        }
        self.store
            .validate_complete(self.key_identity, self.lock_identity)?;
        drop(local);
        drop(guard);
        Ok(value)
    }
    pub(crate) fn acquire_lock(&self) -> Result<ProcessLock, RoutineError> {
        self.store.verify_root()?;
        let file = self.store.open_existing(LOCK_NAME, libc::O_RDWR)?;
        if self.store.exact_identity(LOCK_NAME, &file, 0o600)? != self.lock_identity {
            return Err(error("routine-production-authority-lock-replaced"));
        }
        let guard = ProcessLock::acquire(file)?;
        if self.store.exact_identity(LOCK_NAME, &guard.0, 0o600)? != self.lock_identity
            || read_bounded(&guard.0, LOCK_MARKER.len() as u64)? != LOCK_MARKER
        {
            return Err(error("routine-production-authority-lock-mutated"));
        }
        Ok(guard)
    }
}
