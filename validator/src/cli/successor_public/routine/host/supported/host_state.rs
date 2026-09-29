use super::*;
use sha2::{Digest, Sha256};
use std::os::unix::ffi::OsStrExt;

#[cfg(test)]
thread_local! {
    static AFTER_QUARANTINE_MARKER_BEGIN_FAILPOINT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn fail_after_next_quarantine_marker_begin() {
    AFTER_QUARANTINE_MARKER_BEGIN_FAILPOINT.with(|failpoint| failpoint.set(true));
}

#[cfg(test)]
fn take_after_quarantine_marker_begin_failpoint() -> bool {
    AFTER_QUARANTINE_MARKER_BEGIN_FAILPOINT.with(|failpoint| failpoint.replace(false))
}

#[cfg(not(test))]
fn take_after_quarantine_marker_begin_failpoint() -> bool {
    false
}

impl HostState {
    pub(crate) fn open_existing(home: &Path) -> Result<Self, HostFailure> {
        let home_directory = open_home(home)?;
        let codex = home_directory.open_child(STATE_COMPONENTS[0])?;
        let state_root = codex.open_child(STATE_COMPONENTS[1])?;
        let harness = state_root.open_child(STATE_COMPONENTS[2])?;
        let parent_state_lock = ParentStateLock::shared(&harness)?;
        if harness.stat(BOOTSTRAP_STAGE)?.is_some() {
            return Err(HostFailure::TransitionAmbiguous);
        }
        let state = harness.open_child(STATE_COMPONENTS[3])?;
        verify_ordinary_transition_gate(&harness, &state)?;
        open_existing_state(home_directory, state, parent_state_lock)
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
        let parent_state_lock = ParentStateLock::shared(&base)?;
        let bootstrap_present = base.stat(BOOTSTRAP_STAGE)?.is_some();
        match base.open_child(STATE_COMPONENTS[3]) {
            Ok(_) if bootstrap_present => Err(HostFailure::TransitionAmbiguous),
            Ok(state) => {
                verify_ordinary_transition_gate(&base, &state)?;
                open_existing_state(home_directory, state, parent_state_lock)
            }
            Err(HostFailure::Unavailable)
                if migration_admission::has_transition_evidence(&base)? =>
            {
                Err(HostFailure::TransitionAmbiguous)
            }
            Err(HostFailure::Unavailable) => {
                // Bootstrap changes the complete owner namespace, so it must
                // not run under a reader lock. Release the observation guard,
                // acquire the writer guard, and repeat every decision under
                // that exclusive authority.
                drop(parent_state_lock);
                let parent_state_lock = ParentStateLock::exclusive(&base)?;
                let bootstrap_present = base.stat(BOOTSTRAP_STAGE)?.is_some();
                match base.open_child(STATE_COMPONENTS[3]) {
                    Ok(_) if bootstrap_present => Err(HostFailure::TransitionAmbiguous),
                    Ok(state) => {
                        verify_ordinary_transition_gate(&base, &state)?;
                        open_existing_state(home_directory, state, parent_state_lock)
                    }
                    Err(HostFailure::Unavailable)
                        if migration_admission::has_transition_evidence(&base)? =>
                    {
                        Err(HostFailure::TransitionAmbiguous)
                    }
                    Err(HostFailure::Unavailable) => {
                        bootstrap_new_state(home_directory, base, parent_state_lock)
                    }
                    Err(error) => Err(error),
                }
            }
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
        parent_state_lock: ParentStateLock,
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
            _parent_state_lock: parent_state_lock,
        };
        state.verify_structure()?;
        Ok(state)
    }

    pub(crate) fn verify(&self) -> Result<(), HostFailure> {
        self.verify_structure()?;
        require_adapter_entries(&self.adapter)?;
        match self.adapter.open_child(CONTINUITY_DIRECTORY_NAME) {
            Ok(continuations) => continuity::validate_continuation_directory(&continuations)?,
            Err(HostFailure::Unavailable) => {}
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub(crate) fn verify_structure(&self) -> Result<(), HostFailure> {
        self.home.verify()?;
        self.state.verify()?;
        self.authority.verify()?;
        self.adapter.verify()?;
        self.launch.verify()?;
        verify_ordinary_state_entries(&self.state)?;
        let codex = self.home.open_child(STATE_COMPONENTS[0])?;
        let state_root = codex.open_child(STATE_COMPONENTS[1])?;
        let owner_parent = state_root.open_child(STATE_COMPONENTS[2])?;
        verify_ordinary_parent_evidence(&owner_parent, &self.state)?;
        verify_state_format(&self.state)?;
        require_adapter_shape(&self.adapter)?;
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
        if create {
            self.verify()?;
        } else {
            self.verify_structure()?;
        }
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
        if create {
            self.verify()?;
        } else {
            self.verify_structure()?;
        }
        Ok(Some(HostEventStore {
            store,
            leaf,
            identity,
        }))
    }

    pub(crate) fn verify_event_store(&self, store: &HostEventStore) -> Result<(), HostFailure> {
        self.verify_structure()?;
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
        self.verify_structure()?;
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

pub(super) fn validate_target(home: &Path, target: &Path) -> Result<(), HostFailure> {
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

pub(super) fn validate_event_leaf_name(name: &str) -> Result<(), HostFailure> {
    let digest = name
        .strip_prefix(EVENT_FILE_PREFIX)
        .and_then(|value| value.strip_suffix(EVENT_FILE_SUFFIX))
        .ok_or(HostFailure::Invalid)?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

pub(super) fn open_home(home: &Path) -> Result<AnchoredDirectory, HostFailure> {
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

pub(super) fn open_existing_state(
    home: AnchoredDirectory,
    state: AnchoredDirectory,
    parent_state_lock: ParentStateLock,
) -> Result<HostState, HostFailure> {
    verify_ordinary_state_entries(&state)?;
    verify_state_format(&state)?;
    let authority = state.open_child(AUTHORITY_DIRECTORY)?;
    let adapter = state.open_child(ADAPTER_DIRECTORY)?;
    let launch = state.open_child(LAUNCH_DIRECTORY)?;
    require_entries(&launch, &[])?;
    require_adapter_shape(&adapter)?;
    let lock = adapter.open_regular(LOCK_NAME, libc::O_RDWR, 0o600)?;
    HostState::from_locked(
        home,
        state,
        authority,
        adapter,
        launch,
        lock,
        false,
        parent_state_lock,
    )
}

pub(super) fn bootstrap_new_state(
    home: AnchoredDirectory,
    base: AnchoredDirectory,
    parent_state_lock: ParentStateLock,
) -> Result<HostState, HostFailure> {
    match bootstrap_stage(home, &base, parent_state_lock) {
        Ok(state) => Ok(state),
        Err(HostFailure::TransitionAmbiguous) => Err(HostFailure::TransitionAmbiguous),
        Err(_) if base.open_child(STATE_COMPONENTS[3]).is_ok() => Err(HostFailure::Busy),
        Err(error) => Err(error),
    }
}

fn bootstrap_stage(
    home: AnchoredDirectory,
    base: &AnchoredDirectory,
    parent_state_lock: ParentStateLock,
) -> Result<HostState, HostFailure> {
    let mut staged = build_staged_state(home, base, parent_state_lock, None)?;
    match base.publish_child_exclusive(
        BOOTSTRAP_STAGE,
        STATE_COMPONENTS[3],
        ExclusivePublishSite::BootstrapFresh,
    ) {
        Ok(()) => {}
        Err(ExclusivePublishFailure::BeforeRename) => return Err(HostFailure::Invalid),
        Err(ExclusivePublishFailure::AfterRenameDurabilityUnknown) => {
            return Err(HostFailure::TransitionAmbiguous);
        }
    }
    rebind_state_owner(&mut staged, base, STATE_COMPONENTS[3]);
    staged
        .verify()
        .map_err(|_| HostFailure::TransitionAmbiguous)?;
    Ok(staged)
}

pub(super) fn stage_quarantine_state(
    home: AnchoredDirectory,
    base: &AnchoredDirectory,
    parent_state_lock: ParentStateLock,
    accepted: &super::super::RoutineStateQuarantinePlan,
) -> Result<HostState, HostFailure> {
    build_staged_state(home, base, parent_state_lock, Some(accepted))
}

pub(super) fn open_quarantine_transition_state(
    home: AnchoredDirectory,
    base: &AnchoredDirectory,
    owner: &str,
    parent_state_lock: ParentStateLock,
    accepted: &super::super::RoutineStateQuarantinePlan,
) -> Result<HostState, HostFailure> {
    let state = base.open_child(owner)?;
    let authority = state.open_child(AUTHORITY_DIRECTORY)?;
    let adapter = state.open_child(ADAPTER_DIRECTORY)?;
    let launch = state.open_child(LAUNCH_DIRECTORY)?;
    let lock = adapter.open_regular(LOCK_NAME, libc::O_RDWR, 0o600)?;
    let (lock, lock_identity) = acquire_locked_marker(lock, false)?;
    let staged = HostState {
        home,
        state,
        authority,
        adapter,
        launch,
        lock,
        lock_identity,
        _parent_state_lock: parent_state_lock,
    };
    verify_quarantine_transition_state(&staged, accepted)?;
    Ok(staged)
}

pub(super) fn rebind_state_owner(state: &mut HostState, base: &AnchoredDirectory, owner: &str) {
    state.state.path = base.path.join(owner);
    state.authority.path = state.state.path.join(AUTHORITY_DIRECTORY);
    state.adapter.path = state.state.path.join(ADAPTER_DIRECTORY);
    state.launch.path = state.state.path.join(LAUNCH_DIRECTORY);
}

fn build_staged_state(
    home: AnchoredDirectory,
    base: &AnchoredDirectory,
    parent_state_lock: ParentStateLock,
    transition: Option<&super::super::RoutineStateQuarantinePlan>,
) -> Result<HostState, HostFailure> {
    let (state, created) = base.open_or_create_owned_child(BOOTSTRAP_STAGE)?;
    let allowed = if transition.is_some() {
        &[
            AUTHORITY_DIRECTORY,
            ADAPTER_DIRECTORY,
            LAUNCH_DIRECTORY,
            STATE_FORMAT_NAME,
            QUARANTINE_TRANSITION_MARKER,
        ][..]
    } else {
        &[
            AUTHORITY_DIRECTORY,
            ADAPTER_DIRECTORY,
            LAUNCH_DIRECTORY,
            STATE_FORMAT_NAME,
        ][..]
    };
    require_entries(&state, allowed)?;
    if transition.is_some() && !created && state.stat(QUARANTINE_TRANSITION_MARKER)?.is_none() {
        return Err(HostFailure::Invalid);
    }
    if let Some(accepted) = transition {
        quarantine_transition::begin(&state, accepted)?;
        if take_after_quarantine_marker_begin_failpoint() {
            return Err(HostFailure::Invalid);
        }
    }
    let (adapter, _) = state.open_or_create_owned_child(ADAPTER_DIRECTORY)?;
    let (mut format, created) = state.open_or_create_regular(STATE_FORMAT_NAME, 0o600)?;
    if !created && transition.is_none() {
        return Err(HostFailure::Invalid);
    }
    if created {
        use std::io::Write;
        format
            .write_all(STATE_FORMAT_BYTES)
            .map_err(|_| HostFailure::Invalid)?;
        format.sync_all().map_err(|_| HostFailure::Invalid)?;
    } else {
        verify_state_format(&state)?;
    }
    require_entries(&adapter, &[LOCK_NAME])?;
    let (lock, _) = adapter.open_or_create_regular(LOCK_NAME, 0o600)?;
    let (lock, lock_identity) = acquire_locked_marker(lock, true)?;
    require_entries(&state, allowed)?;
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
        _parent_state_lock: parent_state_lock,
    };
    if let Some(accepted) = transition {
        verify_quarantine_transition_state(&staged, accepted)?;
    } else {
        staged.verify()?;
    }
    Ok(staged)
}

pub(super) fn verify_quarantine_transition_state(
    state: &HostState,
    accepted: &super::super::RoutineStateQuarantinePlan,
) -> Result<(), HostFailure> {
    state.home.verify()?;
    state.state.verify()?;
    state.authority.verify()?;
    state.adapter.verify()?;
    state.launch.verify()?;
    let allowed = match quarantine_transition::classify(&state.state, accepted)? {
        quarantine_transition::SettlementState::Pending => &[
            AUTHORITY_DIRECTORY,
            ADAPTER_DIRECTORY,
            LAUNCH_DIRECTORY,
            STATE_FORMAT_NAME,
            QUARANTINE_TRANSITION_MARKER,
        ][..],
        quarantine_transition::SettlementState::ReceiptStaged => &[
            AUTHORITY_DIRECTORY,
            ADAPTER_DIRECTORY,
            LAUNCH_DIRECTORY,
            STATE_FORMAT_NAME,
            QUARANTINE_TRANSITION_MARKER,
            QUARANTINE_SETTLEMENT_STAGE,
        ][..],
        quarantine_transition::SettlementState::None
        | quarantine_transition::SettlementState::Settled => return Err(HostFailure::Invalid),
    };
    require_entries(&state.state, allowed)?;
    verify_state_format(&state.state)?;
    require_adapter_shape(&state.adapter)?;
    let metadata = state.lock.metadata().map_err(|_| HostFailure::Invalid)?;
    if identity(&metadata) != state.lock_identity
        || state.adapter.stat(LOCK_NAME)? != Some(state.lock_identity)
    {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn verify_ordinary_transition_gate(
    owner_parent: &AnchoredDirectory,
    state: &AnchoredDirectory,
) -> Result<(), HostFailure> {
    match verify_ordinary_parent_evidence(owner_parent, state) {
        Ok(()) => Ok(()),
        Err(_) => Err(HostFailure::TransitionAmbiguous),
    }
}

fn verify_ordinary_parent_evidence(
    owner_parent: &AnchoredDirectory,
    state: &AnchoredDirectory,
) -> Result<(), HostFailure> {
    let quarantine_siblings = quarantine_siblings(owner_parent)?;
    match persisted_settlement(state)? {
        None if quarantine_siblings.is_empty() => Ok(()),
        Some((accepted, quarantine_transition::SettlementState::Settled))
            if quarantine_siblings.len() == 1
                && quarantine_siblings[0] == accepted.quarantine_owner =>
        {
            Ok(())
        }
        None
        | Some((
            _,
            quarantine_transition::SettlementState::None
            | quarantine_transition::SettlementState::Pending
            | quarantine_transition::SettlementState::ReceiptStaged,
        ))
        | Some((_, quarantine_transition::SettlementState::Settled)) => Err(HostFailure::Invalid),
    }
}

fn verify_ordinary_state_entries(state: &AnchoredDirectory) -> Result<(), HostFailure> {
    let allowed = match persisted_settlement_state(state)? {
        quarantine_transition::SettlementState::None => &[
            AUTHORITY_DIRECTORY,
            ADAPTER_DIRECTORY,
            LAUNCH_DIRECTORY,
            STATE_FORMAT_NAME,
        ][..],
        quarantine_transition::SettlementState::Settled => &[
            AUTHORITY_DIRECTORY,
            ADAPTER_DIRECTORY,
            LAUNCH_DIRECTORY,
            STATE_FORMAT_NAME,
            QUARANTINE_TRANSITION_MARKER,
            QUARANTINE_SETTLEMENT_RECEIPT,
        ][..],
        quarantine_transition::SettlementState::Pending
        | quarantine_transition::SettlementState::ReceiptStaged => {
            return Err(HostFailure::Invalid);
        }
    };
    require_entries(state, allowed)
}

fn persisted_settlement_state(
    state: &AnchoredDirectory,
) -> Result<quarantine_transition::SettlementState, HostFailure> {
    Ok(persisted_settlement(state)?
        .map(|(_, settlement)| settlement)
        .unwrap_or(quarantine_transition::SettlementState::None))
}

fn persisted_settlement(
    state: &AnchoredDirectory,
) -> Result<
    Option<(
        super::super::RoutineStateQuarantinePlan,
        quarantine_transition::SettlementState,
    )>,
    HostFailure,
> {
    let Some((accepted, settlement)) = quarantine_transition::classify_present(state)? else {
        return Ok(None);
    };
    if !migration_admission::quarantine_plan_is_semantically_valid(&accepted) {
        return Err(HostFailure::Invalid);
    }
    Ok(Some((accepted, settlement)))
}

fn quarantine_siblings(owner_parent: &AnchoredDirectory) -> Result<Vec<String>, HostFailure> {
    let mut siblings = owner_parent
        .entry_names()?
        .into_iter()
        .filter(|name| name.starts_with("routine-public.quarantine-"))
        .collect::<Vec<_>>();
    siblings.sort();
    Ok(siblings)
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

pub(super) fn require_adapter_entries(adapter: &AnchoredDirectory) -> Result<(), HostFailure> {
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

fn require_adapter_shape(adapter: &AnchoredDirectory) -> Result<(), HostFailure> {
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
        adapter.open_regular(&name, libc::O_RDONLY, 0o600)?;
    }
    Ok(())
}

fn verify_state_format(state: &AnchoredDirectory) -> Result<(), HostFailure> {
    let file = state.open_regular(STATE_FORMAT_NAME, libc::O_RDONLY, 0o600)?;
    let mut bytes = Vec::new();
    use std::io::Read;
    file.take((STATE_FORMAT_BYTES.len() + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| HostFailure::Invalid)?;
    if bytes != STATE_FORMAT_BYTES {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

pub(super) fn read_lock_marker(lock: &File) -> Result<Vec<u8>, HostFailure> {
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

#[cfg(test)]
mod migration_transition_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn fixture(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "hul-bootstrap-transition-{name}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        let home = root.join("home");
        let target = root.join("target");
        let parent = home.join(".codex/state/harness-ultragoal");
        for directory in [
            &root,
            &home,
            &target,
            &home.join(".codex"),
            &home.join(".codex/state"),
            &parent,
        ] {
            fs::create_dir(directory).unwrap();
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
        }
        (root, home, target)
    }

    fn assert_transition_refuses_bootstrap(name: &str) {
        let (root, home, target) = fixture(name);
        let parent = home.join(".codex/state/harness-ultragoal");
        let transition = parent.join(name);
        fs::create_dir(&transition).unwrap();
        fs::set_permissions(&transition, fs::Permissions::from_mode(0o700)).unwrap();
        if name == BOOTSTRAP_STAGE {
            let marker = transition.join(QUARANTINE_TRANSITION_MARKER);
            fs::write(&marker, b"plan-bound-transition").unwrap();
            fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
        }

        assert!(matches!(
            HostState::open_or_bootstrap(&home, &target),
            Err(HostFailure::TransitionAmbiguous)
        ));
        assert!(!parent.join(STATE_COMPONENTS[3]).exists());
        assert!(transition.is_dir());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ordinary_open_cannot_bootstrap_over_prebootstrap_quarantine_evidence() {
        assert_transition_refuses_bootstrap(&format!(
            "routine-public.quarantine-{}",
            "a".repeat(64)
        ));
    }

    #[test]
    fn ordinary_open_cannot_bootstrap_over_a_plan_marked_bootstrap_stage() {
        assert_transition_refuses_bootstrap(BOOTSTRAP_STAGE);
    }

    #[test]
    fn ordinary_bootstrap_upgrades_to_and_retains_the_exclusive_parent_lock() {
        let (root, home, target) = fixture("exclusive-bootstrap");
        let parent_path = home.join(".codex/state/harness-ultragoal");
        let state = HostState::open_or_bootstrap(&home, &target).unwrap();
        let observer =
            AnchoredDirectory::open_absolute(&parent_path, DirectorySecurity::PrivateAuthority)
                .unwrap();

        assert!(matches!(
            ParentStateLock::shared(&observer),
            Err(HostFailure::Busy)
        ));
        assert_eq!(
            fs::read(
                parent_path
                    .join(STATE_COMPONENTS[3])
                    .join(STATE_FORMAT_NAME)
            )
            .unwrap(),
            STATE_FORMAT_BYTES
        );

        drop(state);
        ParentStateLock::shared(&observer).unwrap();
        drop(observer);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ordinary_entry_points_hold_while_the_active_fresh_owner_is_plan_marked() {
        let (root, home, target) = fixture("active-plan-marked");
        let state = HostState::open_or_bootstrap(&home, &target).unwrap();
        drop(state);
        let marker = home
            .join(".codex/state/harness-ultragoal")
            .join(STATE_COMPONENTS[3])
            .join(QUARANTINE_TRANSITION_MARKER);
        fs::write(&marker, b"pending").unwrap();
        fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();

        assert!(matches!(
            HostState::open_existing(&home),
            Err(HostFailure::TransitionAmbiguous)
        ));
        assert!(matches!(
            HostState::open_or_bootstrap(&home, &target),
            Err(HostFailure::TransitionAmbiguous)
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ordinary_bootstrap_reports_post_publish_durability_ambiguity() {
        let (root, home, target) = fixture("bootstrap-publish-ambiguity");
        let parent = home.join(".codex/state/harness-ultragoal");
        fail_after_next_rename(ExclusivePublishSite::BootstrapFresh);

        assert!(matches!(
            HostState::open_or_bootstrap(&home, &target),
            Err(HostFailure::TransitionAmbiguous)
        ));
        assert!(!parent.join(BOOTSTRAP_STAGE).exists());
        assert_eq!(
            fs::read(parent.join(STATE_COMPONENTS[3]).join(STATE_FORMAT_NAME)).unwrap(),
            STATE_FORMAT_BYTES
        );

        fs::remove_dir_all(root).unwrap();
    }
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
