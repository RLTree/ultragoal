use super::*;
use sha2::{Digest, Sha256};
use std::os::unix::ffi::OsStrExt;

impl HostState {
    pub(crate) fn open_existing(home: &Path) -> Result<Self, HostFailure> {
        let home_directory = open_home(home)?;
        let codex = home_directory.open_child(STATE_COMPONENTS[0])?;
        let state_root = codex.open_child(STATE_COMPONENTS[1])?;
        let harness = state_root.open_child(STATE_COMPONENTS[2])?;
        let state = harness.open_child(STATE_COMPONENTS[3])?;
        open_existing_state(home_directory, state)
    }

    pub(crate) fn open_existing_for_target(
        home: &Path,
        target: &Path,
    ) -> Result<Self, HostFailure> {
        validate_target(home, target)?;
        Self::open_existing(home)
    }

    pub(crate) fn open_or_bootstrap(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        let home_directory = open_home(home)?;
        validate_target(home, target)?;
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
        launch: AnchoredDirectory,
        lock: File,
        initialize_marker: bool,
    ) -> Result<Self, HostFailure> {
        let (lock, lock_identity) = acquire_locked_marker(lock, initialize_marker)?;
        let state = Self {
            home,
            state,
            authority,
            adapter,
            launch,
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
        self.launch.verify()?;
        require_entries(
            &self.state,
            &[AUTHORITY_DIRECTORY, ADAPTER_DIRECTORY, LAUNCH_DIRECTORY],
        )?;
        require_adapter_entries(&self.adapter)?;
        match self.adapter.open_child(CONTINUITY_DIRECTORY_NAME) {
            Ok(continuations) => continuity::validate_continuation_directory(&continuations)?,
            Err(HostFailure::Unavailable) => {}
            Err(error) => return Err(error),
        }
        let metadata = self.lock.metadata().map_err(|_| HostFailure::Invalid)?;
        if identity(&metadata) != self.lock_identity
            || self.adapter.stat(LOCK_NAME)? != Some(self.lock_identity)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }

    pub(crate) fn open_event_store(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        source_id: &str,
        create: bool,
    ) -> Result<Option<HostEventStore>, HostFailure> {
        validate_target(&self.home.path, target)?;
        self.verify()?;
        let leaf = event_leaf_name(target, context_id, candidate_id, source_id)?;
        let (file, created) = if create {
            self.adapter.open_or_create_regular(&leaf, 0o600)?
        } else {
            match self.adapter.stat(&leaf)? {
                None => return Ok(None),
                Some(_) => (
                    self.adapter.open_regular(&leaf, libc::O_RDONLY, 0o600)?,
                    false,
                ),
            }
        };
        let identity = super::identity(&file.metadata().map_err(|_| HostFailure::Invalid)?);
        if self.adapter.stat(&leaf)? != Some(identity) {
            return Err(HostFailure::Invalid);
        }
        // SAFETY: geteuid has no preconditions and only reads the process credential.
        let owner = unsafe { libc::geteuid() };
        let store = crate::observability::EventStore::open_descriptor_bound(
            self.adapter
                .file
                .try_clone()
                .map_err(|_| HostFailure::Invalid)?,
            &leaf,
            &file,
            owner,
            0o600,
            context_id,
            candidate_id,
            source_id,
        )
        .map_err(|_| HostFailure::Invalid)?;
        if created {
            self.adapter
                .file
                .sync_all()
                .map_err(|_| HostFailure::Invalid)?;
        }
        self.verify()?;
        Ok(Some(HostEventStore {
            store,
            leaf,
            identity,
        }))
    }

    pub(crate) fn verify_event_store(&self, store: &HostEventStore) -> Result<(), HostFailure> {
        self.verify()?;
        let file = self
            .adapter
            .open_regular(&store.leaf, libc::O_RDONLY, 0o600)?;
        if super::identity(&file.metadata().map_err(|_| HostFailure::Invalid)?) != store.identity
            || self.adapter.stat(&store.leaf)? != Some(store.identity)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }

    pub(crate) fn event_store_absent(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        source_id: &str,
    ) -> Result<bool, HostFailure> {
        self.verify()?;
        let leaf = event_leaf_name(target, context_id, candidate_id, source_id)?;
        Ok(self.adapter.stat(&leaf)?.is_none())
    }

    pub(crate) fn event_checkpoints(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        source_id: &str,
    ) -> Result<Vec<continuity::ContinuationCheckpoint>, HostFailure> {
        if source_id != "successor-runtime" {
            return Err(HostFailure::Invalid);
        }
        let expected_leaf = event_leaf_name(target, context_id, candidate_id, source_id)?;
        let checkpoints = continuity::all_checkpoints(&self.adapter)?
            .into_iter()
            .filter(|checkpoint| {
                checkpoint.is_terminal()
                    && checkpoint.has_current_event_authority()
                    && checkpoint.target() == target.to_str().unwrap_or_default()
                    && checkpoint.context_id() == context_id
                    && checkpoint.candidate_id() == candidate_id
                    && event_leaf_name(
                        target,
                        checkpoint.context_id(),
                        checkpoint.candidate_id(),
                        source_id,
                    )
                    .as_deref()
                        == Ok(expected_leaf.as_str())
            })
            .collect();
        Ok(checkpoints)
    }

    pub(crate) fn all_event_checkpoints(
        &self,
    ) -> Result<Vec<continuity::ContinuationCheckpoint>, HostFailure> {
        Ok(continuity::all_checkpoints(&self.adapter)?
            .into_iter()
            .filter(|checkpoint| {
                checkpoint.is_terminal() && checkpoint.has_current_event_authority()
            })
            .collect())
    }
}

fn validate_target(home: &Path, target: &Path) -> Result<(), HostFailure> {
    let state_root = home.join(STATE_COMPONENTS.join("/"));
    if !target.is_absolute()
        || fs::canonicalize(target).map_err(|_| HostFailure::Invalid)? != target
        || target.starts_with(&state_root)
        || state_root.starts_with(target)
    {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

pub(super) fn event_leaf_name(
    target: &Path,
    context_id: &str,
    candidate_id: &str,
    source_id: &str,
) -> Result<String, HostFailure> {
    let binding =
        crate::observability::EventStore::binding_leaf_name(context_id, candidate_id, source_id)
            .map_err(|_| HostFailure::Invalid)?;
    let mut digest = Sha256::new();
    for value in [target.as_os_str().as_bytes(), binding.as_bytes()] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value);
    }
    Ok(format!(
        "{EVENT_FILE_PREFIX}{:x}{EVENT_FILE_SUFFIX}",
        digest.finalize()
    ))
}

fn validate_event_leaf_name(name: &str) -> Result<(), HostFailure> {
    let digest = name
        .strip_prefix(EVENT_FILE_PREFIX)
        .and_then(|value| value.strip_suffix(EVENT_FILE_SUFFIX))
        .ok_or(HostFailure::Invalid)?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn open_home(home: &Path) -> Result<AnchoredDirectory, HostFailure> {
    if !home.is_absolute() || fs::canonicalize(home).map_err(|_| HostFailure::Unavailable)? != home
    {
        return Err(HostFailure::Invalid);
    }
    AnchoredDirectory::open_absolute(home, DirectorySecurity::HostAncestry)
}

fn open_base(home: &AnchoredDirectory) -> Result<AnchoredDirectory, HostFailure> {
    let (codex, _) = home
        .open_or_create_child_with_security(STATE_COMPONENTS[0], DirectorySecurity::HostAncestry)?;
    let (state, _) = codex
        .open_or_create_child_with_security(STATE_COMPONENTS[1], DirectorySecurity::HostAncestry)?;
    let (authority, _) = state.open_or_create_child_with_security(
        STATE_COMPONENTS[2],
        DirectorySecurity::PrivateAuthority,
    )?;
    Ok(authority)
}

fn open_existing_state(
    home: AnchoredDirectory,
    state: AnchoredDirectory,
) -> Result<HostState, HostFailure> {
    require_entries(
        &state,
        &[AUTHORITY_DIRECTORY, ADAPTER_DIRECTORY, LAUNCH_DIRECTORY],
    )?;
    let authority = state.open_child(AUTHORITY_DIRECTORY)?;
    let adapter = state.open_child(ADAPTER_DIRECTORY)?;
    let launch = state.open_child(LAUNCH_DIRECTORY)?;
    require_entries(&launch, &[])?;
    require_adapter_entries(&adapter)?;
    let lock = adapter.open_regular(LOCK_NAME, libc::O_RDWR, 0o600)?;
    HostState::from_locked(home, state, authority, adapter, launch, lock, false)
}

fn bootstrap_new_state(
    home: AnchoredDirectory,
    base: AnchoredDirectory,
) -> Result<HostState, HostFailure> {
    match bootstrap_stage(home, &base) {
        Ok(state) => Ok(state),
        Err(_) if base.open_child(STATE_COMPONENTS[3]).is_ok() => Err(HostFailure::Busy),
        Err(error) => Err(error),
    }
}

fn bootstrap_stage(
    home: AnchoredDirectory,
    base: &AnchoredDirectory,
) -> Result<HostState, HostFailure> {
    let (state, _) = base.open_or_create_owned_child(BOOTSTRAP_STAGE)?;
    require_entries(
        &state,
        &[AUTHORITY_DIRECTORY, ADAPTER_DIRECTORY, LAUNCH_DIRECTORY],
    )?;
    let (adapter, _) = state.open_or_create_owned_child(ADAPTER_DIRECTORY)?;
    require_entries(&adapter, &[LOCK_NAME])?;
    let (lock, _) = adapter.open_or_create_regular(LOCK_NAME, 0o600)?;
    let (lock, lock_identity) = acquire_locked_marker(lock, true)?;
    require_entries(
        &state,
        &[AUTHORITY_DIRECTORY, ADAPTER_DIRECTORY, LAUNCH_DIRECTORY],
    )?;
    let (authority, _) = state.open_or_create_owned_child(AUTHORITY_DIRECTORY)?;
    require_entries(&authority, &[])?;
    let (launch, _) = state.open_or_create_owned_child(LAUNCH_DIRECTORY)?;
    require_entries(&launch, &[])?;
    let staged = HostState {
        home: home.duplicate()?,
        state,
        authority,
        adapter,
        launch,
        lock,
        lock_identity,
    };
    staged.verify()?;
    base.publish_child_exclusive(BOOTSTRAP_STAGE, STATE_COMPONENTS[3])?;
    drop(staged);
    let state = base.open_child(STATE_COMPONENTS[3])?;
    open_existing_state(home, state)
}

fn require_entries(directory: &AnchoredDirectory, allowed: &[&str]) -> Result<(), HostFailure> {
    if directory
        .entry_names()?
        .iter()
        .all(|name| allowed.contains(&name.as_str()))
    {
        Ok(())
    } else {
        Err(HostFailure::Invalid)
    }
}

fn require_adapter_entries(adapter: &AnchoredDirectory) -> Result<(), HostFailure> {
    let allowed_events = continuity::event_leaf_names(adapter)?;
    for name in adapter.entry_names()? {
        if [
            LOCK_NAME,
            CONTINUITY_CHECKPOINT_NAME,
            CONTINUITY_DIRECTORY_NAME,
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        validate_event_leaf_name(&name)?;
        if !allowed_events.contains(&name) {
            return Err(HostFailure::Invalid);
        }
        adapter.open_regular(&name, libc::O_RDONLY, 0o600)?;
    }
    Ok(())
}

fn read_lock_marker(lock: &File) -> Result<Vec<u8>, HostFailure> {
    let mut marker = lock.try_clone().map_err(|_| HostFailure::Invalid)?;
    marker
        .seek(SeekFrom::Start(0))
        .map_err(|_| HostFailure::Invalid)?;
    let mut bytes = Vec::new();
    marker
        .take((LOCK_MARKER.len() + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| HostFailure::Invalid)?;
    Ok(bytes)
}

fn acquire_locked_marker(
    lock: File,
    initialize_marker: bool,
) -> Result<(File, Identity), HostFailure> {
    let lock_identity = identity(&lock.metadata().map_err(|_| HostFailure::Invalid)?);
    let lock = ProcessLock::acquire(lock)?.0;
    let marker = read_lock_marker(&lock)?;
    if marker != LOCK_MARKER && (!initialize_marker || !LOCK_MARKER.starts_with(&marker)) {
        return Err(HostFailure::Invalid);
    }
    if marker != LOCK_MARKER {
        write_lock_marker(&lock)?;
    }
    if read_lock_marker(&lock)? != LOCK_MARKER {
        return Err(HostFailure::Invalid);
    }
    Ok((lock, lock_identity))
}
