use super::*;

impl FileLedger {
    pub(crate) fn open_or_initialize(root: &Path) -> Result<Self, RoutineError> {
        let store = Store::open(root)?;
        let guard = store.acquire_initial_lock()?;
        let lock_identity = store.exact_identity(LOCK_NAME, &guard.0, 0o600)?;
        let names = store.names()?;
        if names == BTreeSet::from([LOCK_NAME.to_owned()]) {
            let key = store.create_key()?;
            let key_identity = store.exact_identity(KEY_NAME, &key, 0o600)?;
            let key = read_key(&key)?;
            let key_id = sha256(&key.0);
            let authority_id = authority_id(&key_id, store.identity, lock_identity)?;
            let payload = initial_payload(&authority_id, &key_id, store.identity, lock_identity)?;
            store.write_initial_state(&encode(&payload, &key)?)?;
            store.validate_complete(key_identity, lock_identity)?;
        } else if names != complete_names() {
            return Err(error("routine-production-authority-store-incomplete"));
        }
        Self::load_complete(store, guard, lock_identity)
    }
    pub(crate) fn open_existing(root: &Path) -> Result<Self, RoutineError> {
        let store = Store::open(root)?;
        if store.stat_name(LOCK_NAME)?.is_none() {
            return Err(error("routine-production-reuse-authority-missing"));
        }
        let lock = store.open_existing(LOCK_NAME, libc::O_RDWR)?;
        let lock_identity = store.exact_identity(LOCK_NAME, &lock, 0o600)?;
        let guard = ProcessLock::acquire(lock)?;
        if read_bounded(&guard.0, LOCK_MARKER.len() as u64)? != LOCK_MARKER
            || store.names()? != complete_names()
        {
            return Err(error("routine-production-authority-store-incomplete"));
        }
        Self::load_complete(store, guard, lock_identity)
    }
    pub(crate) fn load_complete(
        store: Store,
        guard: ProcessLock,
        lock_identity: FileIdentity,
    ) -> Result<Self, RoutineError> {
        let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY)?;
        let key_identity = store.exact_identity(KEY_NAME, &key_file, 0o600)?;
        let key = read_key(&key_file)?;
        let key_id = sha256(&key.0);
        let authority_id = authority_id(&key_id, store.identity, lock_identity)?;
        let bytes = store.read_state()?;
        let payload = decode(
            &bytes,
            &key,
            &authority_id,
            &key_id,
            store.identity,
            lock_identity,
        )?;
        validate_payload(&payload)?;
        let head_sha256 = sha256(&bytes);
        let state_identity = store.state_identity()?;
        store.validate_complete(key_identity, lock_identity)?;
        drop(guard);
        Ok(Self {
            store,
            key_identity,
            lock_identity,
            key_id,
            authority_id,
            local: Mutex::new(LocalHead {
                generation: payload.generation,
                head_sha256,
                state_identity,
            }),
        })
    }
    pub(crate) fn preauthorize_reuse(
        &self,
        binding: &AuthorityBinding,
        claims: Vec<ReuseArtifactClaim>,
    ) -> Result<ReusePreauthorization, RoutineError> {
        validate_binding(binding)?;
        validate_reuse_claims(binding, &claims)?;
        let authority_id = self.authority_id.clone();
        let binding = binding.clone();
        self.with_payload(false, move |payload, _tick| {
            let Some(record) = payload.protocols.get(&binding.protocol_id) else {
                return Err(error("mediator-production-reuse-not-authenticated"));
            };
            if record.binding != binding
                || record.state != AttemptState::Complete
                || record.artifacts.is_empty()
                || record.artifacts.len() != claims.len()
                || claims.iter().any(|claim| {
                    record.artifacts.get(&claim.artifact_sha256)
                        != Some(&claim.mediator_witness_sha256)
                })
            {
                return Err(error("mediator-production-reuse-not-authenticated"));
            }
            Ok(ReusePreauthorization {
                authority_id,
                binding,
                generation: payload.generation,
                record_sha256: sha256(&canonical(record)?),
                claims,
            })
        })
    }
}
