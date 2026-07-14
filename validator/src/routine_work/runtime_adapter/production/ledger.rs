//! Descriptor-bound durable authority for production routine mediation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::routine_work::digest::{canonical, sha256, valid};
use crate::routine_work::{RoutineError, RoutineErrorId};

const SCHEMA: &str = "RoutineProductionAuthorityLedger-v1";
const KEY_NAME: &str = "routine-authority.key";
const LOCK_NAME: &str = "routine-authority.lock";
const STATE_NAME: &str = "routine-authority.state";
const LOCK_MARKER: &[u8] = b"routine-production-authority-lock-v1\n";
const KEY_BYTES: usize = 32;
const MAX_STATE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RECORDS: usize = 4_096;
const GRANT_TTL_SECONDS: u64 = 300;
const RECOVERY_TTL_SECONDS: u64 = 1_800;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuthorityBinding {
    pub(super) protocol_id: String,
    pub(super) effect_id: String,
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) plan_id: String,
    pub(super) snapshot_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AttemptState {
    Reserved,
    Started,
    Complete,
    Failed,
    Cancelled,
    Incomplete,
}

impl AttemptState {
    fn pending(self) -> bool {
        matches!(self, Self::Reserved | Self::Started)
    }
}

pub(super) struct ReservationSpec {
    pub(super) binding: AuthorityBinding,
    pub(super) request_id: String,
    pub(super) grant_id: String,
    pub(super) recovery_marker: String,
    pub(super) recovery_for: Option<String>,
    pub(super) reuse_only: bool,
    pub(super) reuse_preauthorization: Option<ReusePreauthorization>,
}

pub(super) struct ReuseArtifactClaim {
    pub(super) protocol_id: String,
    pub(super) intent_id: String,
    pub(super) artifact_sha256: String,
    pub(super) result_artifact_sha256: String,
    pub(super) mediator_witness_sha256: String,
}

/// Opaque, one-use authorization issued only after a read-only inspection of
/// one exact Complete record. It intentionally implements neither Clone nor
/// any serialization trait and is consumed by the reservation CAS.
#[must_use = "reuse preauthorization must be consumed by one exact reservation"]
pub(super) struct ReusePreauthorization {
    authority_id: String,
    binding: AuthorityBinding,
    generation: u64,
    record_sha256: String,
    claims: Vec<ReuseArtifactClaim>,
}

#[derive(Clone)]
pub(super) struct ReservationToken {
    pub(super) binding: AuthorityBinding,
    pub(super) request_id: String,
    pub(super) grant_id: String,
    pub(super) recovery_marker: String,
    pub(super) recovery_for: Option<String>,
    pub(super) reuse_only: bool,
    expires_tick: u64,
}

pub(super) struct PendingRecovery {
    pub(super) marker: String,
    pub(super) deadline_tick: u64,
}

pub(super) struct FileAuthorityLedger {
    #[cfg(target_vendor = "apple")]
    inner: supported::FileLedger,
}

impl FileAuthorityLedger {
    pub(super) fn open_or_initialize(root: &Path) -> Result<Self, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return supported::FileLedger::open_or_initialize(root).map(|inner| Self { inner });
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = root;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    /// Opens only an already-complete authority store. Unlike
    /// `open_or_initialize`, this path never creates the lock, key, or state and
    /// is therefore safe for untrusted supplied-reuse authentication.
    pub(super) fn open_existing(root: &Path) -> Result<Self, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return supported::FileLedger::open_existing(root).map(|inner| Self { inner });
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = root;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(super) fn preauthorize_reuse(
        &self,
        binding: &AuthorityBinding,
        claims: Vec<ReuseArtifactClaim>,
    ) -> Result<ReusePreauthorization, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.preauthorize_reuse(binding, claims);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (binding, claims);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(super) fn reserve(&self, spec: ReservationSpec) -> Result<ReservationToken, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.reserve(spec);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = spec;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(super) fn validate_reserved(&self, token: &ReservationToken) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.validate_reserved(token);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = token;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(super) fn prepare_spawn(&self, token: &ReservationToken) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.prepare_spawn(token);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = token;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(super) fn settle(
        &self,
        token: &ReservationToken,
        state: AttemptState,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.settle(token, state, artifacts);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, state, artifacts);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(super) fn authenticates(
        &self,
        token: &ReservationToken,
        digest: &str,
        witness: &str,
    ) -> Result<bool, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.authenticates(token, digest, witness);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, digest, witness);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(super) fn pending_recovery(
        &self,
        binding: &AuthorityBinding,
    ) -> Result<Option<PendingRecovery>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.pending_recovery(binding);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = binding;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    #[cfg(test)]
    pub(super) fn test_expire_pending(
        &self,
        binding: &AuthorityBinding,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.test_expire_pending(binding);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = binding;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    #[cfg(test)]
    pub(super) fn test_seed_capacity(
        &self,
        protocol_effect_count: usize,
        consumed_grant_count: usize,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self
                .inner
                .test_seed_capacity(protocol_effect_count, consumed_grant_count);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (protocol_effect_count, consumed_grant_count);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    #[cfg(test)]
    pub(super) const fn test_capacity_limits() -> (usize, usize) {
        (MAX_RECORDS, MAX_RECORDS * 4)
    }
}

fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

#[cfg(target_vendor = "apple")]
mod supported {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use std::ffi::{CStr, CString, OsStr};
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    type HmacSha256 = Hmac<Sha256>;

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RootIdentity {
        device: u64,
        inode: u64,
        owner: u32,
        mode: u32,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FileIdentity {
        device: u64,
        inode: u64,
        owner: u32,
        mode: u32,
        links: u64,
        length: u64,
        changed_seconds: i64,
        changed_nanos: i64,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ProtocolRecord {
        binding: AuthorityBinding,
        request_id: String,
        grant_id: String,
        recovery_marker: String,
        recovery_for: Option<String>,
        state: AttemptState,
        reuse_only: bool,
        issued_tick: u64,
        expires_tick: u64,
        recovery_deadline_tick: u64,
        artifacts: BTreeMap<String, String>,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Payload {
        schema_version: String,
        authority_id: String,
        key_id: String,
        root_identity: RootIdentity,
        lock_identity: FileIdentity,
        generation: u64,
        previous_head_sha256: String,
        last_tick: u64,
        protocols: BTreeMap<String, ProtocolRecord>,
        effects: BTreeMap<String, String>,
        consumed_grants: BTreeSet<String>,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        payload: Payload,
        hmac_sha256: String,
    }

    struct LedgerKey([u8; KEY_BYTES]);

    struct Store {
        requested_root: PathBuf,
        canonical_root: PathBuf,
        directory: Arc<File>,
        identity: RootIdentity,
    }

    struct ProcessLock(File);

    #[derive(Clone)]
    struct LocalHead {
        generation: u64,
        head_sha256: String,
        state_identity: FileIdentity,
    }

    pub(super) struct FileLedger {
        store: Store,
        key_identity: FileIdentity,
        lock_identity: FileIdentity,
        key_id: String,
        authority_id: String,
        local: Mutex<LocalHead>,
    }

    impl FileLedger {
        pub(super) fn open_or_initialize(root: &Path) -> Result<Self, RoutineError> {
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
                let payload =
                    initial_payload(&authority_id, &key_id, store.identity, lock_identity)?;
                store.write_initial_state(&encode(&payload, &key)?)?;
                store.validate_complete(key_identity, lock_identity)?;
            } else if names != complete_names() {
                return Err(error("routine-production-authority-store-incomplete"));
            }
            Self::load_complete(store, guard, lock_identity)
        }

        pub(super) fn open_existing(root: &Path) -> Result<Self, RoutineError> {
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

        fn load_complete(
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

        pub(super) fn preauthorize_reuse(
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

        pub(super) fn reserve(
            &self,
            spec: ReservationSpec,
        ) -> Result<ReservationToken, RoutineError> {
            validate_spec(&spec)?;
            self.with_payload(true, |payload, tick| {
                if payload.consumed_grants.contains(&spec.grant_id) {
                    return Err(error("routine-production-grant-replayed"));
                }
                let existing = payload.protocols.get(&spec.binding.protocol_id).cloned();
                match (&spec.recovery_for, existing.as_ref()) {
                    (Some(marker), Some(record))
                        if record.state.pending()
                            && record.recovery_marker == *marker
                            && record.binding == spec.binding
                            && tick <= record.recovery_deadline_tick => {}
                    (Some(_), _) => {
                        return Err(error("routine-production-recovery-authority-invalid"));
                    }
                    (None, Some(record))
                        if spec.reuse_only
                            && record.state == AttemptState::Complete
                            && record.binding == spec.binding
                            && !record.artifacts.is_empty() => {}
                    (None, Some(_)) => {
                        return Err(error("routine-production-semantic-effect-replayed"));
                    }
                    (None, None) if !spec.reuse_only => {
                        if payload.effects.contains_key(&spec.binding.effect_id) {
                            return Err(error("routine-production-semantic-effect-replayed"));
                        }
                    }
                    (None, None) => {
                        return Err(error("routine-production-reuse-without-complete-effect"));
                    }
                }
                if let Some(protocol) = payload.effects.get(&spec.binding.effect_id)
                    && protocol != &spec.binding.protocol_id
                {
                    return Err(error("routine-production-effect-protocol-conflict"));
                }
                validate_reservation_capacity(payload, &spec)?;
                if spec.reuse_only {
                    validate_reuse_preauthorization(
                        payload,
                        &spec,
                        spec.reuse_preauthorization
                            .as_ref()
                            .ok_or_else(|| error("routine-production-reservation-spec-invalid"))?,
                        &self.authority_id,
                    )?;
                }
                let expires_tick = tick
                    .checked_add(GRANT_TTL_SECONDS)
                    .ok_or_else(|| error("routine-production-time-overflow"))?;
                let recovery_deadline_tick = tick
                    .checked_add(RECOVERY_TTL_SECONDS)
                    .ok_or_else(|| error("routine-production-time-overflow"))?;
                payload.consumed_grants.insert(spec.grant_id.clone());
                if spec.reuse_only {
                    // A reuse grant is an authorization to read and authenticate
                    // the existing Complete record. It must never replace that
                    // record with a transient state that malformed or forged
                    // external input could later settle as Failed.
                    return Ok(ReservationToken {
                        binding: spec.binding,
                        request_id: spec.request_id,
                        grant_id: spec.grant_id,
                        recovery_marker: spec.recovery_marker,
                        recovery_for: spec.recovery_for,
                        reuse_only: true,
                        expires_tick,
                    });
                }
                let artifacts = existing.map(|record| record.artifacts).unwrap_or_default();
                let record = ProtocolRecord {
                    binding: spec.binding.clone(),
                    request_id: spec.request_id.clone(),
                    grant_id: spec.grant_id.clone(),
                    recovery_marker: spec.recovery_marker.clone(),
                    recovery_for: spec.recovery_for.clone(),
                    state: AttemptState::Reserved,
                    reuse_only: spec.reuse_only,
                    issued_tick: tick,
                    expires_tick,
                    recovery_deadline_tick,
                    artifacts,
                };
                payload.effects.insert(
                    spec.binding.effect_id.clone(),
                    spec.binding.protocol_id.clone(),
                );
                payload
                    .protocols
                    .insert(spec.binding.protocol_id.clone(), record);
                Ok(ReservationToken {
                    binding: spec.binding,
                    request_id: spec.request_id,
                    grant_id: spec.grant_id,
                    recovery_marker: spec.recovery_marker,
                    recovery_for: spec.recovery_for,
                    reuse_only: false,
                    expires_tick,
                })
            })
        }

        pub(super) fn validate_reserved(
            &self,
            token: &ReservationToken,
        ) -> Result<(), RoutineError> {
            self.with_payload(false, |payload, tick| {
                if token.reuse_only {
                    exact_completed_reuse_record(payload, token)?;
                    if tick > token.expires_tick {
                        return Err(error("routine-production-grant-not-reserved"));
                    }
                    return Ok(());
                }
                let record = exact_record(payload, token)?;
                if record.state != AttemptState::Reserved || tick > record.expires_tick {
                    return Err(error("routine-production-grant-not-reserved"));
                }
                Ok(())
            })
        }

        pub(super) fn prepare_spawn(&self, token: &ReservationToken) -> Result<(), RoutineError> {
            if token.reuse_only {
                return Err(error("routine-production-spawn-authority-invalid"));
            }
            self.with_payload(true, |payload, tick| {
                let record = exact_record_mut(payload, token)?;
                if record.reuse_only
                    || record.state != AttemptState::Reserved
                    || tick > record.expires_tick
                {
                    return Err(error("routine-production-spawn-authority-invalid"));
                }
                record.state = AttemptState::Started;
                Ok(())
            })
        }

        pub(super) fn settle(
            &self,
            token: &ReservationToken,
            state: AttemptState,
            artifacts: &BTreeMap<String, String>,
        ) -> Result<(), RoutineError> {
            if state.pending()
                || artifacts
                    .iter()
                    .any(|(digest, witness)| !valid(digest) || !valid(witness))
            {
                return Err(error("routine-production-settlement-invalid"));
            }
            if token.reuse_only {
                if !artifacts.is_empty() {
                    return Err(error("routine-production-settlement-invalid"));
                }
                return self.with_payload(false, |payload, _tick| {
                    exact_completed_reuse_record(payload, token)?;
                    Ok(())
                });
            }
            self.with_payload(true, |payload, _tick| {
                let record = exact_record_mut(payload, token)?;
                let transition_valid = match state {
                    AttemptState::Complete if record.reuse_only => {
                        record.state == AttemptState::Reserved && !record.artifacts.is_empty()
                    }
                    AttemptState::Complete => record.state == AttemptState::Started,
                    AttemptState::Failed | AttemptState::Cancelled | AttemptState::Incomplete => {
                        matches!(record.state, AttemptState::Reserved | AttemptState::Started)
                    }
                    AttemptState::Reserved | AttemptState::Started => false,
                };
                if !transition_valid {
                    return Err(error("routine-production-settlement-transition-invalid"));
                }
                if state == AttemptState::Complete && !record.reuse_only {
                    if artifacts.is_empty() {
                        return Err(error("routine-production-complete-artifacts-missing"));
                    }
                    record.artifacts.clone_from(artifacts);
                }
                record.state = state;
                Ok(())
            })
        }

        pub(super) fn authenticates(
            &self,
            token: &ReservationToken,
            digest: &str,
            witness: &str,
        ) -> Result<bool, RoutineError> {
            if !valid(digest) || !valid(witness) {
                return Ok(false);
            }
            self.with_payload(false, |payload, _tick| {
                let record = if token.reuse_only {
                    exact_completed_reuse_record(payload, token)?
                } else {
                    exact_record(payload, token)?
                };
                Ok(record
                    .artifacts
                    .get(digest)
                    .is_some_and(|value| value == witness))
            })
        }

        pub(super) fn pending_recovery(
            &self,
            binding: &AuthorityBinding,
        ) -> Result<Option<PendingRecovery>, RoutineError> {
            validate_binding(binding)?;
            self.with_payload(false, |payload, tick| {
                let Some(record) = payload.protocols.get(&binding.protocol_id) else {
                    return Ok(None);
                };
                if record.binding != *binding {
                    return Err(error("routine-production-recovery-binding-mismatch"));
                }
                if !record.state.pending() {
                    return Ok(None);
                }
                if tick > record.recovery_deadline_tick {
                    return Err(error("routine-production-recovery-expired"));
                }
                Ok(Some(PendingRecovery {
                    marker: record.recovery_marker.clone(),
                    deadline_tick: record.recovery_deadline_tick,
                }))
            })
        }

        #[cfg(test)]
        pub(super) fn test_expire_pending(
            &self,
            binding: &AuthorityBinding,
        ) -> Result<(), RoutineError> {
            validate_binding(binding)?;
            self.with_payload(true, |payload, tick| {
                let expired = tick
                    .checked_sub(1)
                    .ok_or_else(|| error("routine-production-test-time-underflow"))?;
                let record = payload
                    .protocols
                    .get_mut(&binding.protocol_id)
                    .ok_or_else(|| error("routine-production-reservation-missing"))?;
                if record.binding != *binding || !record.state.pending() {
                    return Err(error("routine-production-test-pending-record-required"));
                }
                record.issued_tick = expired;
                record.expires_tick = expired;
                record.recovery_deadline_tick = expired;
                Ok(())
            })
        }

        #[cfg(test)]
        pub(super) fn test_seed_capacity(
            &self,
            protocol_effect_count: usize,
            consumed_grant_count: usize,
        ) -> Result<(), RoutineError> {
            if protocol_effect_count > MAX_RECORDS
                || consumed_grant_count > MAX_RECORDS.saturating_mul(4)
            {
                return Err(error("routine-production-test-capacity-invalid"));
            }
            self.with_payload(true, |payload, tick| {
                if payload.protocols.len() != payload.effects.len()
                    || payload.protocols.len() > protocol_effect_count
                    || payload.consumed_grants.len() > consumed_grant_count
                {
                    return Err(error("routine-production-test-capacity-invalid"));
                }
                let mut ordinal = 0_u64;
                while payload.protocols.len() < protocol_effect_count {
                    let protocol_id = test_digest("protocol", ordinal);
                    let effect_id = test_digest("effect", ordinal);
                    ordinal = ordinal
                        .checked_add(1)
                        .ok_or_else(|| error("routine-production-test-capacity-invalid"))?;
                    if payload.protocols.contains_key(&protocol_id)
                        || payload.effects.contains_key(&effect_id)
                    {
                        continue;
                    }
                    let grant_id = test_digest("grant", ordinal);
                    let record = ProtocolRecord {
                        binding: AuthorityBinding {
                            protocol_id: protocol_id.clone(),
                            effect_id: effect_id.clone(),
                            context_id: test_digest("context", ordinal),
                            candidate_id: test_digest("candidate", ordinal),
                            plan_id: test_digest("plan", ordinal),
                            snapshot_id: test_digest("snapshot", ordinal),
                        },
                        request_id: test_digest("request", ordinal),
                        grant_id: grant_id.clone(),
                        recovery_marker: test_digest("recovery", ordinal),
                        recovery_for: None,
                        state: AttemptState::Failed,
                        reuse_only: false,
                        issued_tick: tick,
                        expires_tick: tick,
                        recovery_deadline_tick: tick,
                        artifacts: BTreeMap::new(),
                    };
                    payload.effects.insert(effect_id, protocol_id.clone());
                    payload.protocols.insert(protocol_id, record);
                    payload.consumed_grants.insert(grant_id);
                }
                if payload.consumed_grants.len() > consumed_grant_count {
                    return Err(error("routine-production-test-capacity-invalid"));
                }
                let mut ordinal = 0_u64;
                while payload.consumed_grants.len() < consumed_grant_count {
                    payload
                        .consumed_grants
                        .insert(test_digest("consumed", ordinal));
                    ordinal = ordinal
                        .checked_add(1)
                        .ok_or_else(|| error("routine-production-test-capacity-invalid"))?;
                }
                Ok(())
            })
        }

        fn with_payload<T>(
            &self,
            write: bool,
            operation: impl FnOnce(&mut Payload, u64) -> Result<T, RoutineError>,
        ) -> Result<T, RoutineError> {
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
            let mut local = self
                .local
                .lock()
                .map_err(|_| error("routine-production-local-lock-poisoned"))?;
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
            let value = operation(&mut payload, tick)?;
            if write {
                payload.generation = payload
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| error("routine-production-generation-exhausted"))?;
                payload.previous_head_sha256 = head_sha256;
                payload.last_tick = tick;
                // Every write closure operates on an in-memory projection. Do
                // not publish bytes that a subsequent open would reject.
                validate_payload(&payload)?;
                let next = encode(&payload, &key)?;
                self.store.write_atomic_state(&next)?;
                let confirmed = self.store.read_state()?;
                if confirmed != next {
                    return Err(error("routine-production-authority-publish-mismatch"));
                }
                let decoded = decode(
                    &confirmed,
                    &key,
                    &self.authority_id,
                    &self.key_id,
                    self.store.identity,
                    self.lock_identity,
                )?;
                if decoded != payload {
                    return Err(error("routine-production-authority-revalidation-failed"));
                }
                local.generation = payload.generation;
                local.head_sha256 = sha256(&confirmed);
                local.state_identity = self.store.state_identity()?;
            } else {
                local.generation = payload.generation;
                local.head_sha256 = head_sha256;
                local.state_identity = state_identity;
            }
            self.store
                .validate_complete(self.key_identity, self.lock_identity)?;
            drop(local);
            drop(guard);
            Ok(value)
        }

        fn acquire_lock(&self) -> Result<ProcessLock, RoutineError> {
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

    fn exact_record<'a>(
        payload: &'a Payload,
        token: &ReservationToken,
    ) -> Result<&'a ProtocolRecord, RoutineError> {
        let record = payload
            .protocols
            .get(&token.binding.protocol_id)
            .ok_or_else(|| error("routine-production-reservation-missing"))?;
        if record.binding != token.binding
            || record.request_id != token.request_id
            || record.grant_id != token.grant_id
            || record.recovery_marker != token.recovery_marker
            || record.recovery_for != token.recovery_for
            || record.reuse_only != token.reuse_only
            || record.expires_tick != token.expires_tick
        {
            return Err(error("routine-production-reservation-binding-invalid"));
        }
        Ok(record)
    }

    fn exact_record_mut<'a>(
        payload: &'a mut Payload,
        token: &ReservationToken,
    ) -> Result<&'a mut ProtocolRecord, RoutineError> {
        let record = payload
            .protocols
            .get_mut(&token.binding.protocol_id)
            .ok_or_else(|| error("routine-production-reservation-missing"))?;
        if record.binding != token.binding
            || record.request_id != token.request_id
            || record.grant_id != token.grant_id
            || record.recovery_marker != token.recovery_marker
            || record.recovery_for != token.recovery_for
            || record.reuse_only != token.reuse_only
            || record.expires_tick != token.expires_tick
        {
            return Err(error("routine-production-reservation-binding-invalid"));
        }
        Ok(record)
    }

    fn exact_completed_reuse_record<'a>(
        payload: &'a Payload,
        token: &ReservationToken,
    ) -> Result<&'a ProtocolRecord, RoutineError> {
        let record = payload
            .protocols
            .get(&token.binding.protocol_id)
            .ok_or_else(|| error("routine-production-reservation-missing"))?;
        if !token.reuse_only
            || token.recovery_for.is_some()
            || !payload.consumed_grants.contains(&token.grant_id)
            || record.binding != token.binding
            || record.state != AttemptState::Complete
            || record.artifacts.is_empty()
        {
            return Err(error("routine-production-reuse-reservation-invalid"));
        }
        Ok(record)
    }

    fn validate_reservation_capacity(
        payload: &Payload,
        spec: &ReservationSpec,
    ) -> Result<(), RoutineError> {
        let consumed = payload
            .consumed_grants
            .len()
            .checked_add(1)
            .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
        let adds_protocol =
            !spec.reuse_only && !payload.protocols.contains_key(&spec.binding.protocol_id);
        let adds_effect =
            !spec.reuse_only && !payload.effects.contains_key(&spec.binding.effect_id);
        let protocols = payload
            .protocols
            .len()
            .checked_add(usize::from(adds_protocol))
            .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
        let effects = payload
            .effects
            .len()
            .checked_add(usize::from(adds_effect))
            .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
        if consumed > MAX_RECORDS.saturating_mul(4)
            || protocols > MAX_RECORDS
            || effects > MAX_RECORDS
        {
            return Err(error("routine-production-authority-capacity-exhausted"));
        }
        Ok(())
    }

    #[cfg(test)]
    fn test_digest(label: &str, ordinal: u64) -> String {
        sha256(format!("routine-production-test-{label}-{ordinal}").as_bytes())
    }

    impl Store {
        fn open(root: &Path) -> Result<Self, RoutineError> {
            let supplied = root.to_path_buf();
            let path_metadata = fs::symlink_metadata(&supplied)
                .map_err(|_| error("routine-production-authority-root-missing"))?;
            if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
                return Err(error("routine-production-authority-root-invalid"));
            }
            let mut options = OpenOptions::new();
            options.read(true).custom_flags(
                libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            );
            let directory = options
                .open(&supplied)
                .map_err(|_| error("routine-production-authority-root-open-failed"))?;
            let opened = directory
                .metadata()
                .map_err(|_| error("routine-production-authority-root-stat-failed"))?;
            let expected_uid = unsafe { libc::geteuid() };
            if !opened.is_dir()
                || opened.dev() != path_metadata.dev()
                || opened.ino() != path_metadata.ino()
                || opened.uid() != expected_uid
                || path_metadata.uid() != expected_uid
                || opened.mode() & 0o7777 != 0o700
                || path_metadata.mode() & 0o7777 != 0o700
            {
                return Err(error(
                    "routine-production-authority-root-permissions-invalid",
                ));
            }
            let canonical_root = fs::canonicalize(&supplied)
                .map_err(|_| error("routine-production-authority-root-canonicalize-failed"))?;
            if !canonical_root.is_absolute() {
                return Err(error("routine-production-authority-root-not-absolute"));
            }
            let store = Self {
                requested_root: supplied,
                canonical_root,
                directory: Arc::new(directory),
                identity: root_identity(&opened),
            };
            store.verify_root()?;
            Ok(store)
        }

        fn verify_root(&self) -> Result<(), RoutineError> {
            let path = fs::symlink_metadata(&self.requested_root)
                .map_err(|_| error("routine-production-authority-root-replaced"))?;
            let opened = self
                .directory
                .metadata()
                .map_err(|_| error("routine-production-authority-root-replaced"))?;
            if path.file_type().is_symlink()
                || !path.is_dir()
                || root_identity(&path) != self.identity
                || root_identity(&opened) != self.identity
                || path.uid() != unsafe { libc::geteuid() }
                || path.mode() & 0o7777 != 0o700
                || fs::canonicalize(&self.requested_root)
                    .map_err(|_| error("routine-production-authority-root-replaced"))?
                    != self.canonical_root
                || descriptor_path(&self.directory)? != self.canonical_root
            {
                return Err(error("routine-production-authority-root-replaced"));
            }
            Ok(())
        }

        fn acquire_initial_lock(&self) -> Result<ProcessLock, RoutineError> {
            self.verify_root()?;
            let (file, created) = match self.create_exclusive(LOCK_NAME, 0o600) {
                Ok(file) => (file, true),
                Err(value) if value.cause() == "routine-production-authority-entry-exists" => {
                    (self.open_existing(LOCK_NAME, libc::O_RDWR)?, false)
                }
                Err(value) => return Err(value),
            };
            let mut guard = ProcessLock::acquire(file)?;
            if created {
                guard
                    .0
                    .write_all(LOCK_MARKER)
                    .map_err(|_| error("routine-production-authority-lock-write-failed"))?;
                guard
                    .0
                    .sync_all()
                    .map_err(|_| error("routine-production-authority-lock-sync-failed"))?;
                self.directory
                    .sync_all()
                    .map_err(|_| error("routine-production-authority-root-sync-failed"))?;
            }
            if read_bounded(&guard.0, LOCK_MARKER.len() as u64)? != LOCK_MARKER {
                return Err(error("routine-production-authority-lock-invalid"));
            }
            Ok(guard)
        }

        fn create_key(&self) -> Result<File, RoutineError> {
            let mut bytes = [0u8; KEY_BYTES];
            getrandom::fill(&mut bytes)
                .map_err(|_| error("routine-production-authority-random-unavailable"))?;
            let mut file = self.create_exclusive(KEY_NAME, 0o600)?;
            file.write_all(&bytes)
                .map_err(|_| error("routine-production-authority-key-write-failed"))?;
            file.sync_all()
                .map_err(|_| error("routine-production-authority-key-sync-failed"))?;
            self.directory
                .sync_all()
                .map_err(|_| error("routine-production-authority-root-sync-failed"))?;
            Ok(file)
        }

        fn create_exclusive(&self, name: &str, mode: u32) -> Result<File, RoutineError> {
            validate_name(name)?;
            let name = CString::new(name)
                .map_err(|_| error("routine-production-authority-name-invalid"))?;
            let descriptor = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDWR
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_CLOEXEC
                        | libc::O_NOFOLLOW
                        | libc::O_NONBLOCK,
                    mode,
                )
            };
            if descriptor < 0 {
                return Err(match std::io::Error::last_os_error().raw_os_error() {
                    Some(libc::EEXIST) => error("routine-production-authority-entry-exists"),
                    _ => error("routine-production-authority-entry-create-failed"),
                });
            }
            Ok(unsafe { File::from_raw_fd(descriptor) })
        }

        fn open_existing(&self, name: &str, flags: i32) -> Result<File, RoutineError> {
            validate_name(name)?;
            let name = CString::new(name)
                .map_err(|_| error("routine-production-authority-name-invalid"))?;
            let descriptor = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                    flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
                )
            };
            if descriptor < 0 {
                return Err(error("routine-production-authority-entry-open-failed"));
            }
            Ok(unsafe { File::from_raw_fd(descriptor) })
        }

        fn exact_identity(
            &self,
            name: &str,
            file: &File,
            mode: u32,
        ) -> Result<FileIdentity, RoutineError> {
            let opened = file
                .metadata()
                .map_err(|_| error("routine-production-authority-entry-stat-failed"))?;
            let path = self
                .stat_name(name)?
                .ok_or_else(|| error("routine-production-authority-entry-missing"))?;
            let opened = file_identity(&opened);
            if opened != path
                || opened.owner != unsafe { libc::geteuid() }
                || opened.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG)
                || opened.mode & 0o7777 != mode
                || opened.links != 1
            {
                return Err(error("routine-production-authority-entry-identity-invalid"));
            }
            Ok(opened)
        }

        fn stat_name(&self, name: &str) -> Result<Option<FileIdentity>, RoutineError> {
            validate_name(name)?;
            let name = CString::new(name)
                .map_err(|_| error("routine-production-authority-name-invalid"))?;
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            let result = unsafe {
                libc::fstatat(
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            };
            if result != 0 {
                return match std::io::Error::last_os_error().raw_os_error() {
                    Some(libc::ENOENT) => Ok(None),
                    _ => Err(error("routine-production-authority-entry-stat-failed")),
                };
            }
            Ok(Some(stat_identity(&unsafe { stat.assume_init() })))
        }

        fn names(&self) -> Result<BTreeSet<String>, RoutineError> {
            self.verify_root()?;
            let descriptor = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    c".".as_ptr(),
                    libc::O_RDONLY
                        | libc::O_DIRECTORY
                        | libc::O_CLOEXEC
                        | libc::O_NOFOLLOW
                        | libc::O_NONBLOCK,
                )
            };
            if descriptor < 0 {
                return Err(error("routine-production-authority-root-read-failed"));
            }
            let stream = unsafe { libc::fdopendir(descriptor) };
            if stream.is_null() {
                unsafe { libc::close(descriptor) };
                return Err(error("routine-production-authority-root-read-failed"));
            }
            let mut names = BTreeSet::new();
            let result = loop {
                unsafe { *libc::__error() = 0 };
                let entry = unsafe { libc::readdir(stream) };
                if entry.is_null() {
                    break if unsafe { *libc::__error() } == 0 {
                        Ok(names)
                    } else {
                        Err(error("routine-production-authority-root-read-failed"))
                    };
                }
                let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
                if matches!(bytes, b"." | b"..") {
                    continue;
                }
                let name = std::str::from_utf8(bytes)
                    .map_err(|_| error("routine-production-authority-entry-name-invalid"))?;
                if !matches!(name, KEY_NAME | LOCK_NAME | STATE_NAME)
                    || !names.insert(name.to_owned())
                {
                    break Err(error("routine-production-authority-entry-unknown"));
                }
            };
            if unsafe { libc::closedir(stream) } != 0 {
                return Err(error("routine-production-authority-root-read-failed"));
            }
            result
        }

        fn read_state(&self) -> Result<Vec<u8>, RoutineError> {
            let file = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
            let _ = self.exact_identity(STATE_NAME, &file, 0o600)?;
            read_bounded(&file, MAX_STATE_BYTES)
        }

        fn state_identity(&self) -> Result<FileIdentity, RoutineError> {
            let file = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
            self.exact_identity(STATE_NAME, &file, 0o600)
        }

        fn write_initial_state(&self, bytes: &[u8]) -> Result<(), RoutineError> {
            let mut file = self.create_exclusive(STATE_NAME, 0o600)?;
            file.write_all(bytes)
                .map_err(|_| error("routine-production-authority-state-write-failed"))?;
            file.sync_all()
                .map_err(|_| error("routine-production-authority-state-sync-failed"))?;
            self.directory
                .sync_all()
                .map_err(|_| error("routine-production-authority-root-sync-failed"))?;
            let _ = self.exact_identity(STATE_NAME, &file, 0o600)?;
            Ok(())
        }

        fn write_atomic_state(&self, bytes: &[u8]) -> Result<(), RoutineError> {
            if bytes.is_empty() || bytes.len() as u64 > MAX_STATE_BYTES {
                return Err(error("routine-production-authority-state-size-invalid"));
            }
            let temporary = temporary_name()?;
            let mut file = self.create_exclusive(&temporary, 0o600)?;
            let result = (|| {
                file.write_all(bytes)
                    .map_err(|_| error("routine-production-authority-state-write-failed"))?;
                file.sync_all()
                    .map_err(|_| error("routine-production-authority-state-sync-failed"))?;
                let identity = file_identity(
                    &file
                        .metadata()
                        .map_err(|_| error("routine-production-authority-entry-stat-failed"))?,
                );
                if identity.owner != unsafe { libc::geteuid() }
                    || identity.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG)
                    || identity.mode & 0o7777 != 0o600
                    || identity.links != 1
                    || identity.length != bytes.len() as u64
                {
                    return Err(error("routine-production-authority-state-identity-invalid"));
                }
                rename_relative(&self.directory, &temporary, STATE_NAME)?;
                self.directory
                    .sync_all()
                    .map_err(|_| error("routine-production-authority-root-sync-failed"))?;
                Ok(())
            })();
            if result.is_err() {
                let _ = unlink_relative(&self.directory, &temporary);
            }
            result
        }

        fn validate_complete(
            &self,
            key_identity: FileIdentity,
            lock_identity: FileIdentity,
        ) -> Result<(), RoutineError> {
            if self.names()?
                != BTreeSet::from([
                    KEY_NAME.to_owned(),
                    LOCK_NAME.to_owned(),
                    STATE_NAME.to_owned(),
                ])
            {
                return Err(error("routine-production-authority-store-incomplete"));
            }
            let key = self.open_existing(KEY_NAME, libc::O_RDONLY)?;
            let lock = self.open_existing(LOCK_NAME, libc::O_RDONLY)?;
            let state = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
            if self.exact_identity(KEY_NAME, &key, 0o600)? != key_identity
                || self.exact_identity(LOCK_NAME, &lock, 0o600)? != lock_identity
                || read_bounded(&lock, LOCK_MARKER.len() as u64)? != LOCK_MARKER
            {
                return Err(error("routine-production-authority-store-mutated"));
            }
            let _ = self.exact_identity(STATE_NAME, &state, 0o600)?;
            self.verify_root()
        }
    }

    impl ProcessLock {
        fn acquire(file: File) -> Result<Self, RoutineError> {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
                return Err(error("routine-production-authority-lock-acquire-failed"));
            }
            Ok(Self(file))
        }
    }

    impl Drop for ProcessLock {
        fn drop(&mut self) {
            let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
        }
    }

    fn complete_names() -> BTreeSet<String> {
        BTreeSet::from([
            KEY_NAME.to_owned(),
            LOCK_NAME.to_owned(),
            STATE_NAME.to_owned(),
        ])
    }

    fn initial_payload(
        authority_id: &str,
        key_id: &str,
        root_identity: RootIdentity,
        lock_identity: FileIdentity,
    ) -> Result<Payload, RoutineError> {
        Ok(Payload {
            schema_version: SCHEMA.to_owned(),
            authority_id: authority_id.to_owned(),
            key_id: key_id.to_owned(),
            root_identity,
            lock_identity,
            generation: 0,
            previous_head_sha256: sha256(&canonical(&(
                "routine-production-authority-initial-head-v1",
                authority_id,
            ))?),
            last_tick: now_tick()?,
            protocols: BTreeMap::new(),
            effects: BTreeMap::new(),
            consumed_grants: BTreeSet::new(),
        })
    }

    fn authority_id(
        key_id: &str,
        root: RootIdentity,
        lock: FileIdentity,
    ) -> Result<String, RoutineError> {
        canonical(&("routine-production-authority-v1", key_id, root, lock))
            .map(|bytes| sha256(&bytes))
    }

    fn encode(payload: &Payload, key: &LedgerKey) -> Result<Vec<u8>, RoutineError> {
        let payload_bytes = canonical(payload)?;
        let envelope = Envelope {
            payload: payload.clone(),
            hmac_sha256: hmac(&key.0, &payload_bytes)?,
        };
        canonical(&envelope)
    }

    fn decode(
        bytes: &[u8],
        key: &LedgerKey,
        authority_id: &str,
        key_id: &str,
        root_identity: RootIdentity,
        lock_identity: FileIdentity,
    ) -> Result<Payload, RoutineError> {
        let envelope: Envelope = serde_json::from_slice(bytes)
            .map_err(|_| error("routine-production-authority-state-invalid"))?;
        if canonical(&envelope)? != bytes
            || envelope.payload.schema_version != SCHEMA
            || envelope.payload.authority_id != authority_id
            || envelope.payload.key_id != key_id
            || envelope.payload.root_identity != root_identity
            || envelope.payload.lock_identity != lock_identity
            || envelope.hmac_sha256 != hmac(&key.0, &canonical(&envelope.payload)?)?
        {
            return Err(error("routine-production-authority-state-tampered"));
        }
        Ok(envelope.payload)
    }

    fn validate_payload(payload: &Payload) -> Result<(), RoutineError> {
        if !valid(&payload.authority_id)
            || !valid(&payload.key_id)
            || !valid(&payload.previous_head_sha256)
            || payload.protocols.len() > MAX_RECORDS
            || payload.effects.len() > MAX_RECORDS
            || payload.consumed_grants.len() > MAX_RECORDS.saturating_mul(4)
            || payload.protocols.iter().any(|(protocol, record)| {
                protocol != &record.binding.protocol_id
                    || validate_binding(&record.binding).is_err()
                    || !valid(&record.request_id)
                    || !valid(&record.grant_id)
                    || !valid(&record.recovery_marker)
                    || record
                        .recovery_for
                        .as_ref()
                        .is_some_and(|value| !valid(value))
                    || record.issued_tick > record.expires_tick
                    || record.expires_tick > record.recovery_deadline_tick
                    || record
                        .artifacts
                        .iter()
                        .any(|(digest, witness)| !valid(digest) || !valid(witness))
            })
            || payload.effects.iter().any(|(effect, protocol)| {
                !valid(effect)
                    || !valid(protocol)
                    || payload
                        .protocols
                        .get(protocol)
                        .is_none_or(|record| &record.binding.effect_id != effect)
            })
            || payload.consumed_grants.iter().any(|grant| !valid(grant))
        {
            return Err(error("routine-production-authority-state-shape-invalid"));
        }
        Ok(())
    }

    fn validate_spec(spec: &ReservationSpec) -> Result<(), RoutineError> {
        validate_binding(&spec.binding)?;
        if !valid(&spec.request_id)
            || !valid(&spec.grant_id)
            || !valid(&spec.recovery_marker)
            || spec
                .recovery_for
                .as_ref()
                .is_some_and(|value| !valid(value))
            || spec.reuse_only && spec.recovery_for.is_some()
            || spec.reuse_only != spec.reuse_preauthorization.is_some()
        {
            return Err(error("routine-production-reservation-spec-invalid"));
        }
        Ok(())
    }

    fn validate_reuse_claims(
        binding: &AuthorityBinding,
        claims: &[ReuseArtifactClaim],
    ) -> Result<(), RoutineError> {
        let mut intents = BTreeSet::new();
        let mut artifacts = BTreeSet::new();
        if claims.is_empty()
            || claims.iter().any(|claim| {
                claim.protocol_id != binding.protocol_id
                    || !valid(&claim.intent_id)
                    || !valid(&claim.artifact_sha256)
                    || !valid(&claim.result_artifact_sha256)
                    || !valid(&claim.mediator_witness_sha256)
                    || !intents.insert(claim.intent_id.as_str())
                    || !artifacts.insert(claim.artifact_sha256.as_str())
            })
        {
            return Err(error("routine-production-reuse-claim-invalid"));
        }
        Ok(())
    }

    fn validate_reuse_preauthorization(
        payload: &Payload,
        spec: &ReservationSpec,
        authorization: &ReusePreauthorization,
        authority_id: &str,
    ) -> Result<(), RoutineError> {
        let Some(record) = payload.protocols.get(&spec.binding.protocol_id) else {
            return Err(error("routine-production-reuse-preauthorization-stale"));
        };
        if authorization.authority_id != authority_id
            || authorization.binding != spec.binding
            || authorization.generation != payload.generation
            || authorization.record_sha256 != sha256(&canonical(record)?)
            || record.binding != spec.binding
            || record.state != AttemptState::Complete
            || record.artifacts.is_empty()
            || record.artifacts.len() != authorization.claims.len()
            || validate_reuse_claims(&spec.binding, &authorization.claims).is_err()
            || authorization.claims.iter().any(|claim| {
                record.artifacts.get(&claim.artifact_sha256) != Some(&claim.mediator_witness_sha256)
            })
        {
            return Err(error("routine-production-reuse-preauthorization-stale"));
        }
        Ok(())
    }

    fn validate_binding(binding: &AuthorityBinding) -> Result<(), RoutineError> {
        if [
            &binding.protocol_id,
            &binding.effect_id,
            &binding.context_id,
            &binding.candidate_id,
            &binding.plan_id,
            &binding.snapshot_id,
        ]
        .into_iter()
        .all(|value| valid(value))
        {
            Ok(())
        } else {
            Err(error("routine-production-binding-invalid"))
        }
    }

    fn hmac(key: &[u8], bytes: &[u8]) -> Result<String, RoutineError> {
        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|_| error("routine-production-authority-hmac-invalid"))?;
        mac.update(bytes);
        Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
    }

    fn read_key(file: &File) -> Result<LedgerKey, RoutineError> {
        let bytes = read_bounded(file, KEY_BYTES as u64)?;
        let value: [u8; KEY_BYTES] = bytes
            .try_into()
            .map_err(|_| error("routine-production-authority-key-size-invalid"))?;
        Ok(LedgerKey(value))
    }

    fn read_bounded(file: &File, limit: u64) -> Result<Vec<u8>, RoutineError> {
        let metadata = file
            .metadata()
            .map_err(|_| error("routine-production-authority-entry-stat-failed"))?;
        if metadata.len() > limit {
            return Err(error("routine-production-authority-entry-oversize"));
        }
        let mut cloned = file
            .try_clone()
            .map_err(|_| error("routine-production-authority-entry-read-failed"))?;
        cloned
            .seek(SeekFrom::Start(0))
            .map_err(|_| error("routine-production-authority-entry-read-failed"))?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        cloned
            .take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| error("routine-production-authority-entry-read-failed"))?;
        if bytes.len() as u64 > limit || bytes.len() as u64 != metadata.len() {
            return Err(error("routine-production-authority-entry-read-raced"));
        }
        Ok(bytes)
    }

    fn now_tick() -> Result<u64, RoutineError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .map_err(|_| error("routine-production-trusted-time-unavailable"))
    }

    fn temporary_name() -> Result<String, RoutineError> {
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce)
            .map_err(|_| error("routine-production-authority-random-unavailable"))?;
        Ok(format!(
            ".routine-authority-state.tmp.{}",
            nonce
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
    }

    fn validate_name(name: &str) -> Result<(), RoutineError> {
        if matches!(name, KEY_NAME | LOCK_NAME | STATE_NAME)
            || name
                .strip_prefix(".routine-authority-state.tmp.")
                .is_some_and(|suffix| {
                    suffix.len() == 32 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
        {
            Ok(())
        } else {
            Err(error("routine-production-authority-name-invalid"))
        }
    }

    fn rename_relative(directory: &File, from: &str, to: &str) -> Result<(), RoutineError> {
        let from =
            CString::new(from).map_err(|_| error("routine-production-authority-name-invalid"))?;
        let to =
            CString::new(to).map_err(|_| error("routine-production-authority-name-invalid"))?;
        if unsafe {
            libc::renameat(
                directory.as_raw_fd(),
                from.as_ptr(),
                directory.as_raw_fd(),
                to.as_ptr(),
            )
        } != 0
        {
            return Err(error("routine-production-authority-state-publish-failed"));
        }
        Ok(())
    }

    fn unlink_relative(directory: &File, name: &str) -> Result<(), RoutineError> {
        let name =
            CString::new(name).map_err(|_| error("routine-production-authority-name-invalid"))?;
        if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(error("routine-production-authority-temp-cleanup-failed"));
        }
        Ok(())
    }

    fn root_identity(metadata: &fs::Metadata) -> RootIdentity {
        RootIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
            owner: metadata.uid(),
            mode: metadata.mode(),
        }
    }

    fn file_identity(metadata: &fs::Metadata) -> FileIdentity {
        FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
            owner: metadata.uid(),
            mode: metadata.mode(),
            links: metadata.nlink(),
            length: metadata.len(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
        }
    }

    fn stat_identity(stat: &libc::stat) -> FileIdentity {
        FileIdentity {
            device: stat.st_dev as u64,
            inode: stat.st_ino,
            owner: stat.st_uid,
            mode: u32::from(stat.st_mode),
            links: u64::from(stat.st_nlink),
            length: stat.st_size as u64,
            changed_seconds: stat.st_ctime,
            changed_nanos: stat.st_ctime_nsec,
        }
    }

    fn descriptor_path(file: &File) -> Result<PathBuf, RoutineError> {
        let mut bytes = vec![0u8; libc::PATH_MAX as usize];
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, bytes.as_mut_ptr()) } != 0 {
            return Err(error(
                "routine-production-authority-root-descriptor-path-failed",
            ));
        }
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        Ok(PathBuf::from(OsStr::from_bytes(&bytes[..end])))
    }
}
