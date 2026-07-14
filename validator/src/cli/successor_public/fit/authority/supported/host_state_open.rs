use super::*;

impl HostState {
    pub(crate) fn open(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        Self::open_inner(home, target, false)?.ok_or(HostFailure::Unavailable)
    }
    pub(crate) fn open_existing_pending(
        home: &Path,
        target: &Path,
    ) -> Result<Option<Self>, HostFailure> {
        Self::open_inner(home, target, true)
    }
    pub(crate) fn open_inner(
        home: &Path,
        target: &Path,
        existing_pending_only: bool,
    ) -> Result<Option<Self>, HostFailure> {
        if !home.is_absolute()
            || fs::canonicalize(home).map_err(|_| HostFailure::Unavailable)? != home
        {
            return Err(HostFailure::Invalid);
        }
        let mut current = AnchoredDirectory::open_absolute(home, false)?;
        for component in STATE_COMPONENTS {
            current = current.open_child(component, false)?;
        }
        let state_root = current.path.clone();
        let canonical_target = fs::canonicalize(target).map_err(|_| HostFailure::Invalid)?;
        if state_root.starts_with(&canonical_target) || canonical_target.starts_with(&state_root) {
            return Err(HostFailure::Invalid);
        }
        let authority = current.open_child(AUTHORITY_DIRECTORY, true)?;
        let pending = current.open_child(PENDING_DIRECTORY, true)?;
        let target_metadata =
            fs::symlink_metadata(&canonical_target).map_err(|_| HostFailure::Invalid)?;
        if !target_metadata.is_dir() {
            return Err(HostFailure::Invalid);
        }
        let scope_id = digest(
            &serde_json::to_vec(&(SCOPE_DOMAIN, target_metadata.dev(), target_metadata.ino()))
                .map_err(|_| HostFailure::Persistence)?,
        );
        let stem = scope_id
            .strip_prefix("sha256:")
            .ok_or(HostFailure::Invalid)?;
        let pending_name = format!("{stem}.pending.json");
        let lock_name = format!("{stem}.lock");
        if existing_pending_only && stat_at(&pending.file, &pending_name)?.is_none() {
            return Ok(None);
        }
        let lock = if existing_pending_only {
            ProcessLock::acquire_existing(&pending, &lock_name)?
        } else {
            ProcessLock::acquire(&pending, &lock_name)?
        };
        let store = HostStore {
            root: authority.path.clone(),
            authority_file: authority
                .file
                .try_clone()
                .map_err(|_| HostFailure::Persistence)?,
            authority_identity: authority.identity,
            pending_path: pending.path.clone(),
            pending_file: pending
                .file
                .try_clone()
                .map_err(|_| HostFailure::Persistence)?,
            pending_identity: pending.identity,
            lock_name: lock_name.clone(),
            lock_file: lock
                .file
                .try_clone()
                .map_err(|_| HostFailure::Persistence)?,
            lock_identity: lock.identity,
            store_id: digest(STORE_DOMAIN),
        };
        let state = Self {
            authority,
            pending,
            store,
            scope_id,
            pending_name,
            lock_name,
            _lock: lock,
        };
        state.verify()?;
        Ok(Some(state))
    }
    pub(crate) fn verify(&self) -> Result<(), HostFailure> {
        self.authority.verify(true)?;
        self.pending.verify(true)?;
        self._lock.verify(&self.pending, &self.lock_name)?;
        if !self.store.revalidate_protected_root() {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }
    pub(crate) fn verify_after(
        &self,
        outcome: &RepositoryFitProductionOutcome,
    ) -> Result<(), HostFailure> {
        self.verify().map_err(|failure| {
            if outcome.effect_started() {
                HostFailure::Persistence
            } else {
                failure
            }
        })
    }
    pub(crate) fn read_pending(&self) -> Result<Option<PendingRecord>, HostFailure> {
        self.verify()?;
        let Some(path_identity) = stat_at(&self.pending.file, &self.pending_name)? else {
            return Ok(None);
        };
        if !path_identity.safe_regular() {
            return Err(HostFailure::Invalid);
        }
        let file = openat(
            &self.pending.file,
            &self.pending_name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        let opened = identity(&file.metadata().map_err(|_| HostFailure::Invalid)?);
        if opened != path_identity || !opened.safe_regular() {
            return Err(HostFailure::Invalid);
        }
        let mut bytes = Vec::new();
        file.take(MAX_PENDING_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| HostFailure::Invalid)?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_PENDING_BYTES {
            return Err(HostFailure::Invalid);
        }
        if stat_at(&self.pending.file, &self.pending_name)? != Some(opened) {
            return Err(HostFailure::Invalid);
        }
        let envelope: PendingEnvelope =
            serde_json::from_slice(&bytes).map_err(|_| HostFailure::Invalid)?;
        envelope.validate(&self.scope_id, &bytes)?;
        Ok(Some(PendingRecord {
            envelope,
            identity: opened,
        }))
    }
}
