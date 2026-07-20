use super::*;

impl HostState {
    pub(crate) fn open_or_bootstrap(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        let home_directory = open_home(home)?;
        if !target.is_absolute()
            || fs::canonicalize(target).map_err(|_| HostFailure::Invalid)? != target
            || target.starts_with(home.join(STATE_COMPONENTS.join("/")))
            || home.join(STATE_COMPONENTS.join("/")).starts_with(target)
        {
            return Err(HostFailure::Invalid);
        }
        let base = open_base(&home_directory)?;
        match base.open_child(STATE_COMPONENTS[3]) {
            Ok(state) => open_existing_state(home_directory, state),
            Err(HostFailure::Unavailable) => bootstrap_new_state(home_directory, base),
            Err(error) => Err(error),
        }
    }

    fn from_locked(
        home: AnchoredDirectory,
        state: AnchoredDirectory,
        authority: AnchoredDirectory,
        adapter: AnchoredDirectory,
        lock: File,
        created: bool,
    ) -> Result<Self, HostFailure> {
        let lock_identity = identity(&lock.metadata().map_err(|_| HostFailure::Invalid)?);
        let lock = ProcessLock::acquire(lock)?.0;
        if created {
            write_lock_marker(&lock)?;
        }
        let mut marker_file = lock.try_clone().map_err(|_| HostFailure::Invalid)?;
        marker_file
            .seek(SeekFrom::Start(0))
            .map_err(|_| HostFailure::Invalid)?;
        let mut marker = Vec::new();
        marker_file
            .take((LOCK_MARKER.len() + 1) as u64)
            .read_to_end(&mut marker)
            .map_err(|_| HostFailure::Invalid)?;
        if marker != LOCK_MARKER {
            return Err(HostFailure::Invalid);
        }
        let state = Self {
            home,
            state,
            authority,
            adapter,
            lock,
            lock_identity,
        };
        state.verify()?;
        Ok(state)
    }

    pub(crate) fn verify(&self) -> Result<(), HostFailure> {
        self.home.verify()?;
        self.state.verify()?;
        self.authority.verify()?;
        self.adapter.verify()?;
        let metadata = self.lock.metadata().map_err(|_| HostFailure::Invalid)?;
        if identity(&metadata) != self.lock_identity
            || self.adapter.stat(LOCK_NAME)? != Some(self.lock_identity)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }
}

fn open_home(home: &Path) -> Result<AnchoredDirectory, HostFailure> {
    if !home.is_absolute() || fs::canonicalize(home).map_err(|_| HostFailure::Unavailable)? != home
    {
        return Err(HostFailure::Invalid);
    }
    AnchoredDirectory::open_absolute(home)
}

fn open_base(home: &AnchoredDirectory) -> Result<AnchoredDirectory, HostFailure> {
    let (mut current, _) = home.open_or_create_owned_child(STATE_COMPONENTS[0])?;
    for component in &STATE_COMPONENTS[1..3] {
        let (next, _) = current.open_or_create_owned_child(component)?;
        current = next;
    }
    Ok(current)
}

fn open_existing_state(
    home: AnchoredDirectory,
    state: AnchoredDirectory,
) -> Result<HostState, HostFailure> {
    let authority = state.open_child(AUTHORITY_DIRECTORY)?;
    let adapter = state.open_child(ADAPTER_DIRECTORY)?;
    let lock = adapter.open_regular(LOCK_NAME, libc::O_RDWR, 0o600)?;
    HostState::from_locked(home, state, authority, adapter, lock, false)
}

fn bootstrap_new_state(
    home: AnchoredDirectory,
    base: AnchoredDirectory,
) -> Result<HostState, HostFailure> {
    let (state, created) = base.open_or_create_owned_child(STATE_COMPONENTS[3])?;
    if !created {
        return open_existing_state(home, state);
    }
    let (authority, authority_created) = state.open_or_create_owned_child(AUTHORITY_DIRECTORY)?;
    let (adapter, adapter_created) = state.open_or_create_owned_child(ADAPTER_DIRECTORY)?;
    if !authority_created || !adapter_created {
        return Err(HostFailure::Invalid);
    }
    let (lock, created) = adapter.open_or_create_regular(LOCK_NAME, 0o600)?;
    if !created {
        return Err(HostFailure::Invalid);
    }
    HostState::from_locked(home, state, authority, adapter, lock, true)
}
