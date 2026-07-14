use super::*;

pub(crate) const STATE_COMPONENTS: &[&str] =
    &[".codex", "state", "harness-ultragoal", "repository-fit"];
pub(crate) const AUTHORITY_DIRECTORY: &str = "authority";
pub(crate) const PENDING_DIRECTORY: &str = "pending";
pub(crate) const PENDING_SCHEMA: &str = "harness-ultragoal.repository-fit-pending.v1";
pub(crate) const STORE_DOMAIN: &[u8] = b"harness-ultragoal.repository-fit-public-store.v1";
pub(crate) const SCOPE_DOMAIN: &str = "harness-ultragoal.repository-fit-target-scope.v1";
pub(crate) const ENVELOPE_DOMAIN: &str = "harness-ultragoal.repository-fit-pending-envelope.v1";
pub(crate) const NONCE_BYTES: usize = 32;
pub(crate) const MAX_PENDING_BYTES: u64 = 192 * 1024;
pub(crate) const RENAME_NOFOLLOW_ANY: libc::c_uint = 0x10;
pub(crate) const RENAME_RESOLVE_BENEATH: libc::c_uint = 0x20;

pub(crate) fn execute(
    context: &LiveContext,
    prepared: PreparedFitApply,
    home: &Path,
) -> Result<RuntimeOutcome, HostFailure> {
    let state = HostState::open(home, context.worktree_root())?;
    state.verify()?;
    if let Some(pending) = state.read_pending()? {
        return recover_record(context, &state, pending);
    }

    let intent = prepare_recovery_intent(context, &prepared).map_err(|_| HostFailure::Invalid)?;
    let mut nonce = [0_u8; NONCE_BYTES];
    getrandom::fill(&mut nonce).map_err(|_| HostFailure::Random)?;
    let envelope = PendingEnvelope::new(&state.scope_id, &intent.to_machine_bytes(), &nonce)?;
    let pending_identity = state.persist_pending(&envelope)?;
    let nonce = RepositoryFitApplyNonce::new(nonce.to_vec()).map_err(|_| HostFailure::Random)?;
    let outcome = execute_prepared_apply(
        context,
        prepared,
        intent,
        &MonotonicClock,
        &state.store,
        nonce,
    );
    state.verify_after(&outcome)?;
    if !outcome.recovery_required() {
        state
            .remove_pending(pending_identity)
            .map_err(|_| HostFailure::Cleanup)?;
    }
    Ok(production_outcome(outcome))
}

pub(crate) fn recover_pending(
    context: &LiveContext,
    home: &Path,
) -> Result<Option<RuntimeOutcome>, HostFailure> {
    let Some(state) = HostState::open_existing_pending(home, context.worktree_root())? else {
        return Ok(None);
    };
    let Some(pending) = state.read_pending()? else {
        return Ok(None);
    };
    recover_record(context, &state, pending).map(Some)
}

pub(crate) fn recover_record(
    context: &LiveContext,
    state: &HostState,
    pending: PendingRecord,
) -> Result<RuntimeOutcome, HostFailure> {
    let nonce = RepositoryFitApplyNonce::new(decode_hex(&pending.envelope.nonce_hex)?)
        .map_err(|_| HostFailure::Invalid)?;
    let intent = decode_hex(&pending.envelope.recovery_intent_hex)?;
    let outcome = recover_prepared_apply(context, &intent, &MonotonicClock, &state.store, nonce);
    state.verify_after(&outcome)?;
    if !outcome.recovery_required() {
        state
            .remove_pending(pending.identity)
            .map_err(|_| HostFailure::Cleanup)?;
    }
    Ok(production_outcome(outcome))
}

pub(crate) struct MonotonicClock;

impl RepositoryFitTrustedClock for MonotonicClock {
    fn trusted_tick(&self) -> Result<u64, FitAdapterError> {
        let mut value = std::mem::MaybeUninit::<libc::timespec>::uninit();
        if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, value.as_mut_ptr()) } != 0 {
            return Err(FitAdapterError::trusted_clock_unavailable());
        }
        let value = unsafe { value.assume_init() };
        if value.tv_sec < 0 || !(0..1_000_000_000).contains(&value.tv_nsec) {
            return Err(FitAdapterError::trusted_clock_unavailable());
        }
        u64::try_from(value.tv_sec).map_err(|_| FitAdapterError::trusted_clock_unavailable())
    }
}

pub(crate) struct HostStore {
    pub(crate) root: PathBuf,
    pub(crate) authority_file: File,
    pub(crate) authority_identity: FileIdentity,
    pub(crate) pending_path: PathBuf,
    pub(crate) pending_file: File,
    pub(crate) pending_identity: FileIdentity,
    pub(crate) lock_name: String,
    pub(crate) lock_file: File,
    pub(crate) lock_identity: FileIdentity,
    pub(crate) store_id: String,
}

impl RepositoryFitAuthorityStore for HostStore {
    fn protected_root(&self) -> &Path {
        &self.root
    }

    fn store_id(&self) -> &str {
        &self.store_id
    }

    fn revalidate_protected_root(&self) -> bool {
        directory_matches(
            &self.root,
            &self.authority_file,
            self.authority_identity,
            true,
        ) && directory_matches(
            &self.pending_path,
            &self.pending_file,
            self.pending_identity,
            true,
        ) && self
            .lock_file
            .metadata()
            .map(|metadata| identity(&metadata) == self.lock_identity)
            .unwrap_or(false)
            && stat_at(&self.pending_file, &self.lock_name)
                .map(|observed| observed == Some(self.lock_identity))
                .unwrap_or(false)
    }
}

pub(crate) struct HostState {
    pub(crate) authority: AnchoredDirectory,
    pub(crate) pending: AnchoredDirectory,
    pub(crate) store: HostStore,
    pub(crate) scope_id: String,
    pub(crate) pending_name: String,
    pub(crate) lock_name: String,
    pub(crate) _lock: ProcessLock,
}
