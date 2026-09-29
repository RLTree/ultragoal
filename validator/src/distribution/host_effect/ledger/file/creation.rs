impl FileHostEffectLedger {
    pub(in crate::distribution::host_effect) fn create(
        root: &Path,
        ledger_id: String,
    ) -> Result<Self, HostEffectLedgerError> {
        validate_id(&ledger_id)?;
        create_root(root)?;
        let store = Store::open(root)?;
        let lock = store.open_or_create_lock()?;
        let lock_identity = exact_identity(&store, LOCK_NAME, &lock, 0)?;
        let guard = ProcessLock::acquire(lock)?;
        require_lock_identity(&store, &guard, lock_identity)?;
        let (key, key_identity) = store.open_or_create_key()?;
        let key_id = digest(&key.0);
        let initial = initial_payload(&ledger_id, &key_id, lock_identity)?;
        match store.exact_stat(STATE_NAME)? {
            Some(_) => {
                let bytes = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
                let payload = decode_snapshot(&bytes, &key, &ledger_id, &key_id)?;
                require_payload_lock(&payload, lock_identity)?;
                replay(&payload)?;
            }
            None => store.write_atomic(
                STATE_NAME,
                &encode_snapshot(&initial, &key)?,
                &guard,
                lock_identity,
            )?,
        }
        store.validate_complete(lock_identity)?;
        let bytes = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
        let payload = decode_snapshot(&bytes, &key, &ledger_id, &key_id)?;
        require_payload_lock(&payload, lock_identity)?;
        replay(&payload)?;
        let ledger = Self {
            root: store.root.clone(),
            canonical_root: store.canonical_root.clone(),
            directory: Arc::clone(&store.directory),
            directory_identity: store.directory_identity,
            key_identity,
            lock_identity,
            ledger_id,
            local: Mutex::new(ObservedHead {
                initialized: true,
                generation: payload.generation,
                head_sha256: payload.head_sha256,
            }),
            #[cfg(test)]
            lock_open_hook: Mutex::new(None),
            #[cfg(test)]
            key_open_hook: Mutex::new(None),
        };
        ledger.verify_store()?;
        Ok(ledger)
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn open(
        root: &Path,
        ledger_id: String,
    ) -> Result<Self, HostEffectLedgerError> {
        validate_id(&ledger_id)?;
        let store = Store::open(root)?;
        let lock = store.open_existing(LOCK_NAME, 0)?;
        let lock_identity = exact_identity(&store, LOCK_NAME, &lock, 0)?;
        let guard = ProcessLock::acquire(lock)?;
        require_lock_identity(&store, &guard, lock_identity)?;
        store.validate_complete(lock_identity)?;
        let (key, key_identity) = store.open_existing_key(|| Ok(()))?;
        let key_id = digest(&key.0);
        let bytes = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
        let payload = decode_snapshot(&bytes, &key, &ledger_id, &key_id)?;
        require_payload_lock(&payload, lock_identity)?;
        replay(&payload)?;
        let ledger = Self {
            root: store.root.clone(),
            canonical_root: store.canonical_root.clone(),
            directory: Arc::clone(&store.directory),
            directory_identity: store.directory_identity,
            key_identity,
            lock_identity,
            ledger_id,
            local: Mutex::new(ObservedHead {
                initialized: true,
                generation: payload.generation,
                head_sha256: payload.head_sha256,
            }),
            #[cfg(test)]
            lock_open_hook: Mutex::new(None),
            #[cfg(test)]
            key_open_hook: Mutex::new(None),
        };
        ledger.verify_store()?;
        Ok(ledger)
    }

    fn with_snapshot<T>(
        &self,
        operation: impl FnOnce(
            &mut SnapshotPayload,
            &ReplayState,
            &LedgerKey,
        ) -> Result<(T, bool), HostEffectLedgerError>,
    ) -> Result<T, HostEffectLedgerError> {
        let mut local = self.local.lock().map_err(|_| ledger_io())?;
        self.verify_store()?;
        let store = self.store();
        let lock = store.open_existing(LOCK_NAME, 0)?;
        if exact_identity(&store, LOCK_NAME, &lock, 0)? != self.lock_identity {
            return Err(tampered());
        }
        #[cfg(test)]
        self.pause_after_lock_open()?;
        let guard = ProcessLock::acquire(lock)?;
        require_lock_identity(&store, &guard, self.lock_identity)?;
        self.verify_store()?;
        store.validate_complete(self.lock_identity)?;
        let (key, identity) = store.open_existing_key(|| self.after_key_identity())?;
        if identity != self.key_identity {
            return Err(tampered());
        }
        let key_id = digest(&key.0);
        let bytes = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
        let mut payload = decode_snapshot(&bytes, &key, &self.ledger_id, &key_id)?;
        require_payload_lock(&payload, self.lock_identity)?;
        let replayed = replay(&payload)?;
        require_not_rolled_back(&local, &payload)?;
        let (value, changed) = operation(&mut payload, &replayed, &key)?;
        if changed {
            if payload.events.len() > MAX_EVENTS {
                return Err(invalid_record());
            }
            store.write_atomic(
                STATE_NAME,
                &encode_snapshot(&payload, &key)?,
                &guard,
                self.lock_identity,
            )?;
            let published = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
            let confirmed = decode_snapshot(&published, &key, &self.ledger_id, &key_id)?;
            require_payload_lock(&confirmed, self.lock_identity)?;
            replay(&confirmed)?;
            if confirmed != payload {
                return Err(tampered());
            }
        }
        store.validate_complete(self.lock_identity)?;
        local.initialized = true;
        local.generation = payload.generation;
        local.head_sha256.clone_from(&payload.head_sha256);
        self.verify_store()?;
        Ok(value)
    }

    fn store(&self) -> Store {
        Store {
            root: self.root.clone(),
            canonical_root: self.canonical_root.clone(),
            directory: Arc::clone(&self.directory),
            directory_identity: self.directory_identity,
        }
    }

    fn verify_store(&self) -> Result<(), HostEffectLedgerError> {
        let store = self.store();
        store.verify_root()?;
        if store.directory_identity != self.directory_identity {
            return Err(tampered());
        }
        if store.exact_stat(LOCK_NAME)? != Some(self.lock_identity) {
            return Err(tampered());
        }
        Ok(())
    }

    #[cfg(test)]
    fn install_lock_open_hook(&self, hook: LockOpenHook) -> Result<(), HostEffectLedgerError> {
        let mut slot = self.lock_open_hook.lock().map_err(|_| ledger_io())?;
        if slot.is_some() {
            return Err(invalid_record());
        }
        *slot = Some(hook);
        Ok(())
    }
}
