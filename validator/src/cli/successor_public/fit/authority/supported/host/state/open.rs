use super::*;

impl HostState {
    pub(crate) fn open(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        match Self::open_complete(home, target, false)? {
            Some(state) => Ok(state),
            None => Self::initialize(home, target),
        }
    }
    pub(crate) fn open_existing_pending(
        home: &Path,
        target: &Path,
    ) -> Result<Option<Self>, HostFailure> {
        Self::open_complete(home, target, true)
    }
    fn open_complete(
        home: &Path,
        target: &Path,
        existing_pending_only: bool,
    ) -> Result<Option<Self>, HostFailure> {
        let Some(mut current) = open_host_state_base(home)? else {
            return Ok(None);
        };
        for component in FIT_STATE_COMPONENTS {
            let Some(child) = current.open_child_optional(component, true)? else {
                return Ok(None);
            };
            current = child;
        }
        let authority = current.open_child_optional(AUTHORITY_DIRECTORY, true)?;
        let pending = current.open_child_optional(PENDING_DIRECTORY, true)?;
        let (Some(authority), Some(pending)) = (authority, pending) else {
            return Ok(None);
        };
        Self::assemble(
            current.path,
            authority,
            pending,
            target,
            existing_pending_only,
        )
    }

    fn initialize(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        let Some(mut current) = open_host_state_base(home)? else {
            return Err(HostFailure::Unavailable);
        };
        reject_target_overlap(&prospective_state_root(&current.path)?, target)?;
        for component in FIT_STATE_COMPONENTS {
            current = current.open_or_create_owned_child(component)?;
        }
        let authority = current.open_or_create_owned_child(AUTHORITY_DIRECTORY)?;
        let pending = current.open_or_create_owned_child(PENDING_DIRECTORY)?;
        Self::assemble(current.path, authority, pending, target, false)?.ok_or(HostFailure::Invalid)
    }

    fn assemble(
        state_root: PathBuf,
        authority: AnchoredDirectory,
        pending: AnchoredDirectory,
        target: &Path,
        existing_pending_only: bool,
    ) -> Result<Option<Self>, HostFailure> {
        let canonical_target = fs::canonicalize(target).map_err(|_| HostFailure::Invalid)?;
        reject_target_overlap(&state_root, &canonical_target)?;
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

fn open_host_state_base(home: &Path) -> Result<Option<AnchoredDirectory>, HostFailure> {
    if !home.is_absolute() {
        return Err(HostFailure::Invalid);
    }
    let Ok(canonical_home) = fs::canonicalize(home) else {
        return Ok(None);
    };
    if canonical_home != home {
        return Err(HostFailure::Invalid);
    }
    let mut current = AnchoredDirectory::open_absolute(home, false)?;
    for component in HOST_STATE_COMPONENTS {
        let Some(child) = current.open_child_optional(component, false)? else {
            return Ok(None);
        };
        current = child;
    }
    Ok(Some(current))
}

fn prospective_state_root(base: &Path) -> Result<PathBuf, HostFailure> {
    let canonical_base = fs::canonicalize(base).map_err(|_| HostFailure::Invalid)?;
    if canonical_base != base {
        return Err(HostFailure::Invalid);
    }
    Ok(FIT_STATE_COMPONENTS
        .iter()
        .fold(canonical_base, |path, component| path.join(component)))
}

fn reject_target_overlap(state_root: &Path, target: &Path) -> Result<(), HostFailure> {
    let canonical_target = fs::canonicalize(target).map_err(|_| HostFailure::Invalid)?;
    let target_metadata =
        fs::symlink_metadata(&canonical_target).map_err(|_| HostFailure::Invalid)?;
    if !target_metadata.is_dir()
        || state_root.starts_with(&canonical_target)
        || canonical_target.starts_with(state_root)
    {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}
