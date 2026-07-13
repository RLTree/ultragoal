use crate::cli::successor::ExitClass;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome};
use crate::context::LiveContext;
use crate::repository_fit::{AdapterErrorId, PreparedFitApply, RepositoryFitProductionOutcome};
use std::path::Path;

pub(super) fn execute(
    context: &LiveContext,
    prepared: PreparedFitApply,
    home: &Path,
) -> RuntimeOutcome {
    #[cfg(target_vendor = "apple")]
    {
        return supported::execute(context, prepared, home).unwrap_or_else(host_failure);
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (context, prepared, home);
        host_failure(HostFailure::Unsupported)
    }
}

pub(super) fn recover_pending(
    context: &LiveContext,
    home: &Path,
) -> Result<Option<RuntimeOutcome>, RuntimeOutcome> {
    #[cfg(target_vendor = "apple")]
    {
        return match supported::recover_pending(context, home) {
            Ok(outcome) => Ok(outcome),
            Err(HostFailure::Unavailable) => Ok(None),
            Err(failure) => Err(host_failure(failure)),
        };
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (context, home);
        Ok(None)
    }
}

pub(super) fn host_state_unavailable() -> RuntimeOutcome {
    host_failure(HostFailure::Unavailable)
}

fn production_outcome(outcome: RepositoryFitProductionOutcome) -> RuntimeOutcome {
    let class = match outcome.status() {
        "applied" | "idempotent" | "recovered" => ExitClass::Success,
        "rolled_back" | "interrupted" | "ambiguous" => ExitClass::ActionableFinding,
        _ => match outcome.error_id() {
            Some(AdapterErrorId::ContextStale | AdapterErrorId::StalePlan) => {
                ExitClass::ActionableFinding
            }
            Some(AdapterErrorId::InvalidPlanRecord) => ExitClass::InvalidInvocation,
            Some(
                AdapterErrorId::AcceptanceMismatch
                | AdapterErrorId::PlanConflict
                | AdapterErrorId::ApplyPermitMissing
                | AdapterErrorId::ApplyPermitInvalid
                | AdapterErrorId::ApplyPermitExpired
                | AdapterErrorId::ApplyPermitReplayed
                | AdapterErrorId::ApplyLeaseInvalid,
            ) => ExitClass::BlockedAuthority,
            Some(AdapterErrorId::UnsupportedHost) => ExitClass::UnsupportedCapability,
            Some(
                AdapterErrorId::ApplyMutationScopeViolation
                | AdapterErrorId::ApplyRolledBack
                | AdapterErrorId::ApplyOutcomeAmbiguous,
            ) => ExitClass::ActionableFinding,
            Some(
                AdapterErrorId::InvalidTemplateCatalog
                | AdapterErrorId::TargetUnavailable
                | AdapterErrorId::ProjectionFailed
                | AdapterErrorId::EffectFailed
                | AdapterErrorId::ApplyOutcomeInvalid,
            )
            | None => ExitClass::InternalFailure,
        },
    };
    match outcome.to_machine_bytes() {
        Ok(machine) => RuntimeOutcome::payload(
            class,
            machine,
            format!(
                "repository fit {} result={} effect_started={} rollback_complete={}",
                outcome.status(),
                outcome.result_id(),
                outcome.effect_started(),
                outcome.rollback_complete()
            ),
        ),
        Err(_) => host_failure(HostFailure::Persistence),
    }
}

#[derive(Clone, Copy, Debug)]
enum HostFailure {
    Unsupported,
    Unavailable,
    Invalid,
    Random,
    Persistence,
    Cleanup,
}

fn host_failure(failure: HostFailure) -> RuntimeOutcome {
    let (class, id, cause, repair, effect, ceiling) = match failure {
        HostFailure::Unsupported => (
            ExitClass::UnsupportedCapability,
            DiagnosticId::DownstreamToolUnavailable,
            "the public repository-fit writer is supported only on a Darwin host",
            "run fit plan and verify on this host, or use the supported Darwin runtime for apply",
            "none",
            "repository-fit apply remains unavailable on this host",
        ),
        HostFailure::Unavailable => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "preprovisioned repository-fit host authority state is unavailable",
            "install or repair the owner-only Harness Ultragoal host state, then rerun the exact accepted plan",
            "none",
            "no workspace effect is authorized or performed",
        ),
        HostFailure::Invalid => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "repository-fit host authority state is aliased, substituted, malformed, or unsafe",
            "repair the installed owner-only host state without replacing its authority ledger, then retry",
            "none",
            "no new workspace effect is authorized",
        ),
        HostFailure::Random => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "the operating system did not provide a production repository-fit nonce",
            "restore the host random source and recompute the accepted fit request",
            "none",
            "no workspace effect is authorized or performed",
        ),
        HostFailure::Persistence => (
            ExitClass::InternalFailure,
            DiagnosticId::ProjectionFailed,
            "repository-fit authority or recovery state could not be durably reconciled",
            "preserve the owner-only state and diagnose it before any retry",
            "workspace_write_not_acknowledged",
            "repository-fit outcome and dependent claims remain withheld",
        ),
        HostFailure::Cleanup => (
            ExitClass::ActionableFinding,
            DiagnosticId::AuthorityRequired,
            "the workspace outcome returned but its durable recovery envelope could not be safely retired",
            "preserve the target and host state, then rerun fit apply to enter exact recovery",
            "workspace_write_may_have_occurred",
            "repository-fit success and dependent claims remain withheld",
        ),
    };
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            cause,
            "HCT-FIT public production authority",
            repair,
            effect,
            "ultragoal --json fit apply --plan <plan> --accept-plan <sha256>",
            ceiling,
        ),
    )
}

#[cfg(target_vendor = "apple")]
mod supported {
    use super::{HostFailure, production_outcome};
    use crate::cli::successor::runtime::RuntimeOutcome;
    use crate::context::LiveContext;
    use crate::repository_fit::{
        FitAdapterError, PreparedFitApply, RepositoryFitApplyNonce, RepositoryFitAuthorityStore,
        RepositoryFitProductionOutcome, RepositoryFitTrustedClock, digest, execute_prepared_apply,
        prepare_recovery_intent, recover_prepared_apply, valid_digest,
    };
    use serde::{Deserialize, Serialize};
    use std::ffi::CString;
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    const STATE_COMPONENTS: &[&str] = &[".codex", "state", "harness-ultragoal", "repository-fit"];
    const AUTHORITY_DIRECTORY: &str = "authority";
    const PENDING_DIRECTORY: &str = "pending";
    const PENDING_SCHEMA: &str = "harness-ultragoal.repository-fit-pending.v1";
    const STORE_DOMAIN: &[u8] = b"harness-ultragoal.repository-fit-public-store.v1";
    const SCOPE_DOMAIN: &str = "harness-ultragoal.repository-fit-target-scope.v1";
    const ENVELOPE_DOMAIN: &str = "harness-ultragoal.repository-fit-pending-envelope.v1";
    const NONCE_BYTES: usize = 32;
    const MAX_PENDING_BYTES: u64 = 192 * 1024;
    const RENAME_NOFOLLOW_ANY: libc::c_uint = 0x10;
    const RENAME_RESOLVE_BENEATH: libc::c_uint = 0x20;

    pub(super) fn execute(
        context: &LiveContext,
        prepared: PreparedFitApply,
        home: &Path,
    ) -> Result<RuntimeOutcome, HostFailure> {
        let state = HostState::open(home, context.worktree_root())?;
        state.verify()?;
        if let Some(pending) = state.read_pending()? {
            return recover_record(context, &state, pending);
        }

        let intent =
            prepare_recovery_intent(context, &prepared).map_err(|_| HostFailure::Invalid)?;
        let mut nonce = [0_u8; NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| HostFailure::Random)?;
        let envelope = PendingEnvelope::new(&state.scope_id, &intent.to_machine_bytes(), &nonce)?;
        let pending_identity = state.persist_pending(&envelope)?;
        let nonce =
            RepositoryFitApplyNonce::new(nonce.to_vec()).map_err(|_| HostFailure::Random)?;
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

    pub(super) fn recover_pending(
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

    fn recover_record(
        context: &LiveContext,
        state: &HostState,
        pending: PendingRecord,
    ) -> Result<RuntimeOutcome, HostFailure> {
        let nonce = RepositoryFitApplyNonce::new(decode_hex(&pending.envelope.nonce_hex)?)
            .map_err(|_| HostFailure::Invalid)?;
        let intent = decode_hex(&pending.envelope.recovery_intent_hex)?;
        let outcome =
            recover_prepared_apply(context, &intent, &MonotonicClock, &state.store, nonce);
        state.verify_after(&outcome)?;
        if !outcome.recovery_required() {
            state
                .remove_pending(pending.identity)
                .map_err(|_| HostFailure::Cleanup)?;
        }
        Ok(production_outcome(outcome))
    }

    struct MonotonicClock;

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

    struct HostStore {
        root: PathBuf,
        authority_file: File,
        authority_identity: FileIdentity,
        pending_path: PathBuf,
        pending_file: File,
        pending_identity: FileIdentity,
        lock_name: String,
        lock_file: File,
        lock_identity: FileIdentity,
        store_id: String,
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

    struct HostState {
        authority: AnchoredDirectory,
        pending: AnchoredDirectory,
        store: HostStore,
        scope_id: String,
        pending_name: String,
        lock_name: String,
        _lock: ProcessLock,
    }

    impl HostState {
        fn open(home: &Path, target: &Path) -> Result<Self, HostFailure> {
            Self::open_inner(home, target, false)?.ok_or(HostFailure::Unavailable)
        }

        fn open_existing_pending(home: &Path, target: &Path) -> Result<Option<Self>, HostFailure> {
            Self::open_inner(home, target, true)
        }

        fn open_inner(
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
            if state_root.starts_with(&canonical_target)
                || canonical_target.starts_with(&state_root)
            {
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

        fn verify(&self) -> Result<(), HostFailure> {
            self.authority.verify(true)?;
            self.pending.verify(true)?;
            self._lock.verify(&self.pending, &self.lock_name)?;
            if !self.store.revalidate_protected_root() {
                return Err(HostFailure::Invalid);
            }
            Ok(())
        }

        fn verify_after(
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

        fn read_pending(&self) -> Result<Option<PendingRecord>, HostFailure> {
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

        fn persist_pending(&self, envelope: &PendingEnvelope) -> Result<FileIdentity, HostFailure> {
            self.verify()?;
            if stat_at(&self.pending.file, &self.pending_name)?.is_some() {
                return Err(HostFailure::Invalid);
            }
            let bytes = envelope.canonical_bytes()?;
            if bytes.is_empty() || bytes.len() as u64 > MAX_PENDING_BYTES {
                return Err(HostFailure::Persistence);
            }
            let mut random = [0_u8; 16];
            getrandom::fill(&mut random).map_err(|_| HostFailure::Random)?;
            let temporary = format!("{}.tmp-{}", self.pending_name, encode_hex(&random));
            let mut file = openat(
                &self.pending.file,
                &temporary,
                libc::O_WRONLY
                    | libc::O_CREAT
                    | libc::O_EXCL
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
                0o600,
            )?;
            let result = (|| {
                if unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
                    return Err(HostFailure::Persistence);
                }
                let created = identity(&file.metadata().map_err(|_| HostFailure::Persistence)?);
                if !created.safe_regular() || created.size != 0 {
                    return Err(HostFailure::Invalid);
                }
                file.write_all(&bytes)
                    .map_err(|_| HostFailure::Persistence)?;
                file.sync_all().map_err(|_| HostFailure::Persistence)?;
                let written = identity(&file.metadata().map_err(|_| HostFailure::Persistence)?);
                if !created.same_object(written)
                    || written.size != bytes.len() as u64
                    || stat_at(&self.pending.file, &temporary)? != Some(written)
                {
                    return Err(HostFailure::Invalid);
                }
                rename_exclusive(&self.pending.file, &temporary, &self.pending_name)?;
                sync_directory(&self.pending.file)?;
                let published = stat_at(&self.pending.file, &self.pending_name)?
                    .ok_or(HostFailure::Persistence)?;
                if published != written || !published.safe_regular() {
                    return Err(HostFailure::Invalid);
                }
                Ok(published)
            })();
            if result.is_err() {
                let _ = unlink_at(&self.pending.file, &temporary);
                let _ = sync_directory(&self.pending.file);
            }
            result
        }

        fn remove_pending(&self, expected: FileIdentity) -> Result<(), HostFailure> {
            self.verify()?;
            if stat_at(&self.pending.file, &self.pending_name)? != Some(expected) {
                return Err(HostFailure::Invalid);
            }
            unlink_at(&self.pending.file, &self.pending_name)?;
            sync_directory(&self.pending.file)?;
            if stat_at(&self.pending.file, &self.pending_name)?.is_some() {
                return Err(HostFailure::Persistence);
            }
            self.verify()
        }
    }

    struct PendingRecord {
        envelope: PendingEnvelope,
        identity: FileIdentity,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct PendingEnvelope {
        schema_version: String,
        target_scope_id: String,
        recovery_intent_hex: String,
        nonce_hex: String,
        binding_sha256: String,
    }

    impl PendingEnvelope {
        fn new(scope: &str, intent: &[u8], nonce: &[u8]) -> Result<Self, HostFailure> {
            if !valid_digest(scope) || nonce.len() != NONCE_BYTES || intent.is_empty() {
                return Err(HostFailure::Invalid);
            }
            let recovery_intent_hex = encode_hex(intent);
            let nonce_hex = encode_hex(nonce);
            let binding_sha256 = envelope_binding(scope, &recovery_intent_hex, &nonce_hex)?;
            Ok(Self {
                schema_version: PENDING_SCHEMA.to_owned(),
                target_scope_id: scope.to_owned(),
                recovery_intent_hex,
                nonce_hex,
                binding_sha256,
            })
        }

        fn canonical_bytes(&self) -> Result<Vec<u8>, HostFailure> {
            serde_json::to_vec(self).map_err(|_| HostFailure::Persistence)
        }

        fn validate(&self, scope: &str, bytes: &[u8]) -> Result<(), HostFailure> {
            if self.schema_version != PENDING_SCHEMA
                || self.target_scope_id != scope
                || !valid_digest(&self.binding_sha256)
                || decode_hex(&self.nonce_hex)?.len() != NONCE_BYTES
                || decode_hex(&self.recovery_intent_hex)?.is_empty()
                || envelope_binding(scope, &self.recovery_intent_hex, &self.nonce_hex)?
                    != self.binding_sha256
                || self.canonical_bytes()? != bytes
            {
                return Err(HostFailure::Invalid);
            }
            Ok(())
        }
    }

    fn envelope_binding(scope: &str, intent: &str, nonce: &str) -> Result<String, HostFailure> {
        serde_json::to_vec(&(ENVELOPE_DOMAIN, scope, intent, nonce))
            .map(|bytes| digest(&bytes))
            .map_err(|_| HostFailure::Persistence)
    }

    struct AnchoredDirectory {
        path: PathBuf,
        file: File,
        identity: FileIdentity,
    }

    impl AnchoredDirectory {
        fn open_absolute(path: &Path, exact_owner_only: bool) -> Result<Self, HostFailure> {
            let encoded =
                CString::new(path.as_os_str().as_bytes()).map_err(|_| HostFailure::Invalid)?;
            let descriptor = unsafe {
                libc::open(
                    encoded.as_ptr(),
                    libc::O_RDONLY
                        | libc::O_DIRECTORY
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC
                        | libc::O_NONBLOCK,
                )
            };
            if descriptor < 0 {
                return Err(HostFailure::Unavailable);
            }
            let file = unsafe { File::from_raw_fd(descriptor) };
            Self::finish(path.to_path_buf(), file, exact_owner_only)
        }

        fn open_child(&self, name: &str, exact_owner_only: bool) -> Result<Self, HostFailure> {
            self.verify(false)?;
            let file = openat(
                &self.file,
                name,
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
                0,
            )?;
            Self::finish(self.path.join(name), file, exact_owner_only)
        }

        fn finish(path: PathBuf, file: File, exact_owner_only: bool) -> Result<Self, HostFailure> {
            let metadata = file.metadata().map_err(|_| HostFailure::Invalid)?;
            let identity = identity(&metadata);
            if !metadata.is_dir()
                || identity.uid != unsafe { libc::geteuid() }
                || identity.mode & 0o022 != 0
                || (exact_owner_only && identity.mode != 0o700)
                || fs::canonicalize(&path).map_err(|_| HostFailure::Invalid)? != path
            {
                return Err(HostFailure::Invalid);
            }
            let value = Self {
                path,
                file,
                identity,
            };
            value.verify(exact_owner_only)?;
            Ok(value)
        }

        fn verify(&self, exact_owner_only: bool) -> Result<(), HostFailure> {
            if !directory_matches(&self.path, &self.file, self.identity, exact_owner_only) {
                return Err(HostFailure::Invalid);
            }
            Ok(())
        }

        fn open_identity(path: &Path) -> Result<FileIdentity, HostFailure> {
            let encoded =
                CString::new(path.as_os_str().as_bytes()).map_err(|_| HostFailure::Invalid)?;
            let descriptor = unsafe {
                libc::open(
                    encoded.as_ptr(),
                    libc::O_RDONLY
                        | libc::O_DIRECTORY
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC
                        | libc::O_NONBLOCK,
                )
            };
            if descriptor < 0 {
                return Err(HostFailure::Invalid);
            }
            let file = unsafe { File::from_raw_fd(descriptor) };
            file.metadata()
                .map(|metadata| identity(&metadata))
                .map_err(|_| HostFailure::Invalid)
        }
    }

    fn directory_matches(
        path: &Path,
        file: &File,
        expected: FileIdentity,
        exact_owner_only: bool,
    ) -> bool {
        let Ok(metadata) = file.metadata() else {
            return false;
        };
        let Ok(reopened) = AnchoredDirectory::open_identity(path) else {
            return false;
        };
        let observed = identity(&metadata);
        observed.same_directory(expected)
            && reopened.same_directory(expected)
            && metadata.is_dir()
            && observed.uid == unsafe { libc::geteuid() }
            && observed.mode & 0o022 == 0
            && (!exact_owner_only || observed.mode == 0o700)
            && fs::canonicalize(path)
                .map(|canonical| canonical == path)
                .unwrap_or(false)
    }

    struct ProcessLock {
        file: File,
        identity: FileIdentity,
    }

    impl ProcessLock {
        fn acquire(directory: &AnchoredDirectory, name: &str) -> Result<Self, HostFailure> {
            let prior = stat_at(&directory.file, name)?;
            if prior.is_some_and(|identity| !identity.safe_regular() || identity.size != 0) {
                return Err(HostFailure::Invalid);
            }
            let (file, created) = match prior {
                Some(_) => (
                    openat(
                        &directory.file,
                        name,
                        libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                        0,
                    )?,
                    false,
                ),
                None => (
                    openat(
                        &directory.file,
                        name,
                        libc::O_RDWR
                            | libc::O_CREAT
                            | libc::O_EXCL
                            | libc::O_NOFOLLOW
                            | libc::O_CLOEXEC
                            | libc::O_NONBLOCK,
                        0o600,
                    )?,
                    true,
                ),
            };
            Self::finish(directory, name, prior, file, created)
        }

        fn acquire_existing(
            directory: &AnchoredDirectory,
            name: &str,
        ) -> Result<Self, HostFailure> {
            let prior = stat_at(&directory.file, name)?.ok_or(HostFailure::Invalid)?;
            if !prior.safe_regular() || prior.size != 0 {
                return Err(HostFailure::Invalid);
            }
            let file = openat(
                &directory.file,
                name,
                libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                0,
            )?;
            Self::finish(directory, name, Some(prior), file, false)
        }

        fn finish(
            directory: &AnchoredDirectory,
            name: &str,
            prior: Option<FileIdentity>,
            file: File,
            created: bool,
        ) -> Result<Self, HostFailure> {
            if created && unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
                return Err(HostFailure::Persistence);
            }
            let identity = identity(&file.metadata().map_err(|_| HostFailure::Invalid)?);
            if !identity.safe_regular()
                || identity.size != 0
                || prior.is_some_and(|prior| prior != identity)
                || stat_at(&directory.file, name)? != Some(identity)
                || unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
            {
                return Err(HostFailure::Invalid);
            }
            if created {
                sync_directory(&directory.file)?;
            }
            Ok(Self { file, identity })
        }

        fn verify(&self, directory: &AnchoredDirectory, name: &str) -> Result<(), HostFailure> {
            if identity(&self.file.metadata().map_err(|_| HostFailure::Invalid)?) != self.identity
                || stat_at(&directory.file, name)? != Some(self.identity)
            {
                return Err(HostFailure::Invalid);
            }
            Ok(())
        }
    }

    impl Drop for ProcessLock {
        fn drop(&mut self) {
            unsafe {
                libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct FileIdentity {
        device: u64,
        inode: u64,
        uid: u32,
        mode: u32,
        links: u64,
        size: u64,
        kind: u32,
    }

    impl FileIdentity {
        fn safe_regular(self) -> bool {
            self.kind == libc::S_IFREG as u32
                && self.uid == unsafe { libc::geteuid() }
                && self.mode == 0o600
                && self.links == 1
                && self.size <= MAX_PENDING_BYTES
        }

        fn same_object(self, other: Self) -> bool {
            self.device == other.device
                && self.inode == other.inode
                && self.uid == other.uid
                && self.mode == other.mode
                && self.links == other.links
                && self.kind == other.kind
        }

        fn same_directory(self, other: Self) -> bool {
            self.device == other.device
                && self.inode == other.inode
                && self.uid == other.uid
                && self.mode == other.mode
                && self.kind == other.kind
                && self.kind == libc::S_IFDIR as u32
        }
    }

    fn identity(metadata: &fs::Metadata) -> FileIdentity {
        FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
            uid: metadata.uid(),
            mode: metadata.mode() & 0o7777,
            links: metadata.nlink(),
            size: metadata.size(),
            kind: metadata.mode() & libc::S_IFMT as u32,
        }
    }

    fn stat_at(directory: &File, name: &str) -> Result<Option<FileIdentity>, HostFailure> {
        let encoded = CString::new(name).map_err(|_| HostFailure::Invalid)?;
        let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                encoded.as_ptr(),
                value.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
                Ok(None)
            } else {
                Err(HostFailure::Invalid)
            };
        }
        let value = unsafe { value.assume_init() };
        Ok(Some(FileIdentity {
            device: value.st_dev as u64,
            inode: value.st_ino as u64,
            uid: value.st_uid,
            mode: value.st_mode as u32 & 0o7777,
            links: value.st_nlink as u64,
            size: u64::try_from(value.st_size).map_err(|_| HostFailure::Invalid)?,
            kind: value.st_mode as u32 & libc::S_IFMT as u32,
        }))
    }

    fn openat(
        directory: &File,
        name: &str,
        flags: libc::c_int,
        mode: libc::mode_t,
    ) -> Result<File, HostFailure> {
        let encoded = CString::new(name).map_err(|_| HostFailure::Invalid)?;
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                encoded.as_ptr(),
                flags,
                mode as libc::c_uint,
            )
        };
        if descriptor < 0 {
            return Err(HostFailure::Unavailable);
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }

    fn rename_exclusive(directory: &File, old: &str, new: &str) -> Result<(), HostFailure> {
        let old = CString::new(old).map_err(|_| HostFailure::Invalid)?;
        let new = CString::new(new).map_err(|_| HostFailure::Invalid)?;
        let flags =
            libc::RENAME_EXCL as libc::c_uint | RENAME_NOFOLLOW_ANY | RENAME_RESOLVE_BENEATH;
        if unsafe {
            libc::renameatx_np(
                directory.as_raw_fd(),
                old.as_ptr(),
                directory.as_raw_fd(),
                new.as_ptr(),
                flags,
            )
        } != 0
        {
            return Err(HostFailure::Persistence);
        }
        Ok(())
    }

    fn unlink_at(directory: &File, name: &str) -> Result<(), HostFailure> {
        let encoded = CString::new(name).map_err(|_| HostFailure::Invalid)?;
        if unsafe { libc::unlinkat(directory.as_raw_fd(), encoded.as_ptr(), 0) } != 0 {
            return Err(HostFailure::Persistence);
        }
        Ok(())
    }

    fn sync_directory(directory: &File) -> Result<(), HostFailure> {
        if unsafe { libc::fsync(directory.as_raw_fd()) } != 0 {
            return Err(HostFailure::Persistence);
        }
        Ok(())
    }

    fn encode_hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(HEX[(byte >> 4) as usize] as char);
            output.push(HEX[(byte & 0x0f) as usize] as char);
        }
        output
    }

    fn decode_hex(value: &str) -> Result<Vec<u8>, HostFailure> {
        if value.is_empty() || value.len() % 2 != 0 || value.len() > MAX_PENDING_BYTES as usize * 2
        {
            return Err(HostFailure::Invalid);
        }
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let high = hex_digit(pair[0])?;
                let low = hex_digit(pair[1])?;
                Ok((high << 4) | low)
            })
            .collect()
    }

    fn hex_digit(value: u8) -> Result<u8, HostFailure> {
        match value {
            b'0'..=b'9' => Ok(value - b'0'),
            b'a'..=b'f' => Ok(value - b'a' + 10),
            _ => Err(HostFailure::Invalid),
        }
    }
}
