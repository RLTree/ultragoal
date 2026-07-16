use super::*;

impl FileLedger {
    pub(crate) fn with_payload<T>(
        &self,
        local: &mut LocalHead,
        write: bool,
        operation: impl FnOnce(&mut Payload, u64) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        self.transition_payload(local, |payload, tick, _head| {
            operation(payload, tick).map(|value| (value, write))
        })?
        .into_result()
    }

    pub(crate) fn transition_payload<T>(
        &self,
        local: &mut LocalHead,
        operation: impl FnOnce(&mut Payload, u64, &str) -> Result<(T, bool), RoutineError>,
    ) -> Result<DurableWrite<T>, RoutineError> {
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
        let (value, write) = operation(&mut payload, tick, &head_sha256)?;
        if write {
            payload.generation = payload
                .generation
                .checked_add(1)
                .ok_or_else(|| error("routine-production-generation-exhausted"))?;
            payload.previous_head_sha256 = head_sha256;
            payload.last_tick = tick;
            validate_payload(&payload)?;
            let next = encode(&payload, &key)?;
            match self.store.write_atomic_state(&next) {
                StatePublication::Committed => {}
                StatePublication::Precommit => {
                    drop(guard);
                    return Ok(DurableWrite::Precommit(value));
                }
                StatePublication::Ambiguous => {
                    drop(guard);
                    return Ok(DurableWrite::Ambiguous(value));
                }
            }
            let confirmed = match self.store.read_state() {
                Ok(confirmed) => confirmed,
                Err(_) => return Ok(DurableWrite::Ambiguous(value)),
            };
            if confirmed != next {
                return Ok(DurableWrite::Ambiguous(value));
            }
            let decoded = match decode(
                &confirmed,
                &key,
                &self.authority_id,
                &self.key_id,
                self.store.identity,
                self.lock_identity,
            ) {
                Ok(decoded) => decoded,
                Err(_) => return Ok(DurableWrite::Ambiguous(value)),
            };
            if decoded != payload {
                return Ok(DurableWrite::Ambiguous(value));
            }
            let confirmed_identity = match self.store.state_identity() {
                Ok(identity) => identity,
                Err(_) => return Ok(DurableWrite::Ambiguous(value)),
            };
            local.generation = payload.generation;
            local.head_sha256 = sha256(&confirmed);
            local.state_identity = confirmed_identity;
        } else {
            local.generation = payload.generation;
            local.head_sha256 = head_sha256;
            local.state_identity = state_identity;
        }
        if self
            .store
            .validate_complete(self.key_identity, self.lock_identity)
            .is_err()
        {
            return if write {
                Ok(DurableWrite::Ambiguous(value))
            } else {
                Err(error("routine-production-authority-store-incomplete"))
            };
        }
        drop(guard);
        Ok(DurableWrite::Committed(value))
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
