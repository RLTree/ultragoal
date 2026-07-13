use super::ledger::{
    ExecutionTerminalProof, FileAuthorityIdentity, FileIdentity, FileLock, entry_exists, hmac,
    open_safe_directory, openat, publish_file, read_directory_names, read_json_file,
    safe_file_identity, sha256, sync_directory,
};
use super::{EvaluationError, PromotionReviewAuthority};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};

const STATE_NAME: &str = "promotion-review.state";
const ANCHOR_NAME: &str = "promotion-review.anchor.journal";
const LOCK_NAME: &str = "promotion-review.lock";
const MAX_LEDGER_BYTES: u64 = 1024 * 1024;
const MAX_ANCHOR_JOURNAL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ANCHOR_RECORD_BYTES: usize = 1024 * 1024;
const ANCHOR_GENESIS: &[u8] = b"promotion-anchor-journal-genesis";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionLedgerBinding {
    authority_id: String,
    reviewer_id: String,
    review_session_id: String,
    live_context_id: String,
    baseline_candidate_id: String,
    candidate_id: String,
    baseline_run_sha256: String,
    candidate_run_sha256: String,
    baseline_execution_session_id: String,
    candidate_execution_session_id: String,
    baseline_execution_head_sha256: String,
    candidate_execution_head_sha256: String,
}

impl PromotionLedgerBinding {
    pub(crate) fn from_terminal_proofs(
        authority_id: impl Into<String>,
        reviewer_id: impl Into<String>,
        review_session_id: impl Into<String>,
        baseline: ExecutionTerminalProof,
        candidate: ExecutionTerminalProof,
    ) -> Result<Self, PromotionLedgerError> {
        let value = Self {
            authority_id: authority_id.into(),
            reviewer_id: reviewer_id.into(),
            review_session_id: review_session_id.into(),
            live_context_id: candidate.binding.live_context_id.clone(),
            baseline_candidate_id: baseline.binding.candidate_id.clone(),
            candidate_id: candidate.binding.candidate_id.clone(),
            baseline_run_sha256: baseline.run_sha256,
            candidate_run_sha256: candidate.run_sha256,
            baseline_execution_session_id: baseline.binding.execution_session_id,
            candidate_execution_session_id: candidate.binding.execution_session_id,
            baseline_execution_head_sha256: baseline.ledger_head_sha256,
            candidate_execution_head_sha256: candidate.ledger_head_sha256,
        };
        if baseline.binding.live_context_id != value.live_context_id
            || value.baseline_candidate_id == value.candidate_id
            || !super::valid_identifier(&value.authority_id)
            || !super::valid_identifier(&value.reviewer_id)
            || value.authority_id == value.reviewer_id
            || !super::valid_sha256(&value.review_session_id)
            || value.review_session_id == value.baseline_execution_session_id
            || value.review_session_id == value.candidate_execution_session_id
            || value.baseline_execution_session_id == value.candidate_execution_session_id
            || [
                value.live_context_id.as_str(),
                value.baseline_candidate_id.as_str(),
                value.candidate_id.as_str(),
                value.baseline_run_sha256.as_str(),
                value.candidate_run_sha256.as_str(),
                value.baseline_execution_head_sha256.as_str(),
                value.candidate_execution_head_sha256.as_str(),
            ]
            .iter()
            .any(|item| !super::valid_sha256(item))
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-binding-invalid",
            ));
        }
        Ok(value)
    }

    fn digest(&self) -> String {
        sha256(&serde_json::to_vec(self).expect("promotion binding serializes"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PromotionLedgerState {
    Ready,
    Issued {
        binding_sha256: String,
        attestation_sha256: String,
    },
    Consumed {
        binding_sha256: String,
        review_id: String,
        attestation_sha256: String,
    },
    RecoveryRequired {
        causal_code: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionLedgerError {
    code: &'static str,
}

impl PromotionLedgerError {
    const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for PromotionLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for PromotionLedgerError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ReviewSnapshotCore {
    schema_version: String,
    generation: u64,
    previous_head_sha256: String,
    key_id: String,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: PromotionLedgerBinding,
    state: PromotionLedgerState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ReviewSnapshotPayload {
    core: ReviewSnapshotCore,
    anchor_observation: FileIdentity,
    anchor_length: u64,
    anchor_head_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatedReviewSnapshot {
    payload: ReviewSnapshotPayload,
    mac_sha256: String,
    head_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ReviewAnchorRecordPayload {
    schema_version: String,
    prior_anchor_head_sha256: String,
    core: ReviewSnapshotCore,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatedReviewAnchorRecord {
    payload: ReviewAnchorRecordPayload,
    mac_sha256: String,
    head_sha256: String,
}

struct CurrentReviewSnapshot {
    snapshot: AuthenticatedReviewSnapshot,
    partial_tail_from: Option<u64>,
    observed_state_head_sha256: String,
    observed_anchor: FileIdentity,
}

#[derive(Debug)]
pub struct FilePromotionReviewLedger {
    root_path: PathBuf,
    root: File,
    root_identity: FileIdentity,
    lock: File,
    lock_identity: FileIdentity,
    anchor: File,
    anchor_authority: FileAuthorityIdentity,
    key: [u8; 32],
    key_id: String,
    binding: PromotionLedgerBinding,
    expected_head: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingReviewPublication {
    name: String,
    generation: u64,
    identity: FileIdentity,
}

impl FilePromotionReviewLedger {
    pub(crate) fn initialize(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: PromotionLedgerBinding,
    ) -> Result<Self, PromotionLedgerError> {
        validate_binding(&binding)?;
        let root_path = root.as_ref().to_path_buf();
        let (root, root_identity) = open_safe_directory(&root_path)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-root-unsafe"))?;
        ensure_entries(&root_path, &root, true)?;
        let lock = open_review_lock(&root, true)?;
        let lock_identity = safe_file_identity(&lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-unsafe"))?;
        let anchor = open_review_anchor(&root, true)?;
        let anchor_authority = safe_file_identity(&anchor)
            .map_err(map_storage)?
            .authority();
        let key_id = sha256(&key);
        let core = ReviewSnapshotCore {
            schema_version: "PromotionReviewLedger-v1".to_owned(),
            generation: 0,
            previous_head_sha256: sha256(b"promotion-review-genesis"),
            key_id: key_id.clone(),
            lock_identity,
            anchor_authority,
            binding: binding.clone(),
            state: PromotionLedgerState::Ready,
        };
        let record = authenticate_anchor_record(
            ReviewAnchorRecordPayload {
                schema_version: "PromotionReviewAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: sha256(ANCHOR_GENESIS),
                core: core.clone(),
            },
            &key,
        )?;
        require_named_review_lock_identity(&root, lock_identity)?;
        require_named_review_anchor_authority(&root, anchor_authority)?;
        let _guard = FileLock::exclusive(&lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        require_named_review_lock_identity(&root, lock_identity)?;
        require_named_review_anchor_authority(&root, anchor_authority)?;
        if entry_exists(&root, STATE_NAME).map_err(map_storage)? {
            return Err(PromotionLedgerError::new("promotion-ledger-already-exists"));
        }
        let (anchor_observation, anchor_length) = append_anchor_record(&anchor, &record)?;
        let snapshot = authenticate_snapshot(
            ReviewSnapshotPayload {
                core,
                anchor_observation,
                anchor_length,
                anchor_head_sha256: record.head_sha256,
            },
            &key,
        )?;
        publish_file(&root, STATE_NAME, &snapshot, 0).map_err(map_storage)?;
        sync_directory(&root).map_err(map_storage)?;
        drop(_guard);
        let ledger = Self {
            root_path,
            root,
            root_identity,
            lock,
            lock_identity,
            anchor,
            anchor_authority,
            key,
            key_id,
            binding,
            expected_head: snapshot.head_sha256.clone(),
        };
        test_final_validation_pause(&ledger.root_path);
        let _guard = FileLock::exclusive(&ledger.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        ledger.require_published_current(&snapshot)?;
        drop(_guard);
        Ok(ledger)
    }

    pub(crate) fn open(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: PromotionLedgerBinding,
    ) -> Result<Self, PromotionLedgerError> {
        validate_binding(&binding)?;
        let root_path = root.as_ref().to_path_buf();
        let (root, root_identity) = open_safe_directory(&root_path)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-root-unsafe"))?;
        let lock = open_review_lock(&root, false)?;
        let lock_identity = safe_file_identity(&lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-unsafe"))?;
        let anchor = open_review_anchor(&root, false)?;
        let anchor_authority = safe_file_identity(&anchor)
            .map_err(map_storage)?
            .authority();
        let key_id = sha256(&key);
        require_named_review_lock_identity(&root, lock_identity)?;
        require_named_review_anchor_authority(&root, anchor_authority)?;
        let (pending, current) = {
            let _guard = FileLock::exclusive(&lock)
                .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
            require_named_review_lock_identity(&root, lock_identity)?;
            require_named_review_anchor_authority(&root, anchor_authority)?;
            let pending = ensure_entries(&root_path, &root, false)?;
            let current = read_current(
                &root,
                &anchor,
                lock_identity,
                anchor_authority,
                &key,
                &key_id,
                &binding,
            )?;
            validate_pending_generation(pending.as_ref(), &current.snapshot)?;
            (pending, current)
        };
        let ledger = Self {
            root_path,
            root,
            root_identity,
            lock,
            lock_identity,
            anchor,
            anchor_authority,
            key,
            key_id,
            binding,
            expected_head: current.snapshot.head_sha256.clone(),
        };
        test_final_validation_pause(&ledger.root_path);
        let _guard = FileLock::exclusive(&ledger.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        ledger.require_unchanged_current(pending.as_ref(), &current)?;
        drop(_guard);
        Ok(ledger)
    }

    pub fn inspect(&self) -> Result<PromotionLedgerState, PromotionLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        let state = current.snapshot.payload.core.state.clone();
        test_final_validation_pause(&self.root_path);
        self.require_unchanged_current(pending.as_ref(), &current)?;
        Ok(state)
    }

    pub fn require_recovery(
        &mut self,
        causal_code: impl Into<String>,
    ) -> Result<(), PromotionLedgerError> {
        let causal_code = causal_code.into();
        if !super::valid_identifier(&causal_code) {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-causal-code-invalid",
            ));
        }
        self.mutate(|state| match state {
            PromotionLedgerState::Ready | PromotionLedgerState::Issued { .. } => {
                Ok((PromotionLedgerState::RecoveryRequired { causal_code }, ()))
            }
            _ => Err(PromotionLedgerError::new(
                "promotion-ledger-recovery-transition-refused",
            )),
        })
    }

    fn issue_bound_attestation(
        &mut self,
        binding_sha256: &str,
    ) -> Result<String, PromotionLedgerError> {
        if !super::valid_sha256(binding_sha256) {
            return Err(PromotionLedgerError::new(
                "promotion-attestation-binding-invalid",
            ));
        }
        let attestation = hmac(
            format!(
                "promotion-attestation-v1|{}|{}|{}|{}",
                binding_sha256,
                self.binding.digest(),
                self.binding.reviewer_id,
                self.binding.review_session_id,
            )
            .as_bytes(),
            &self.key,
        )
        .map_err(map_storage)?;
        self.mutate(|state| match state {
            PromotionLedgerState::Ready => Ok((
                PromotionLedgerState::Issued {
                    binding_sha256: binding_sha256.to_owned(),
                    attestation_sha256: attestation.clone(),
                },
                attestation,
            )),
            PromotionLedgerState::Issued {
                binding_sha256: existing_binding,
                attestation_sha256,
            } if existing_binding == binding_sha256 && attestation_sha256 == attestation => Ok((
                PromotionLedgerState::Issued {
                    binding_sha256: existing_binding,
                    attestation_sha256: attestation_sha256.clone(),
                },
                attestation_sha256,
            )),
            _ => Err(PromotionLedgerError::new(
                "promotion-attestation-issuance-refused",
            )),
        })
    }

    fn consume_attestation(
        &mut self,
        binding_sha256: &str,
        reviewer_id: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> Result<bool, PromotionLedgerError> {
        let expected_review_id =
            sha256(format!("promotion-review|{binding_sha256}|{attestation_sha256}").as_bytes());
        if reviewer_id != self.binding.reviewer_id
            || !super::valid_sha256(binding_sha256)
            || !super::valid_sha256(review_id)
            || !super::valid_sha256(attestation_sha256)
            || review_id != expected_review_id
        {
            return Ok(false);
        }
        self.mutate(|state| match state {
            PromotionLedgerState::Issued {
                binding_sha256: issued_binding,
                attestation_sha256: issued_attestation,
            } if issued_binding == binding_sha256 && issued_attestation == attestation_sha256 => {
                Ok((
                    PromotionLedgerState::Consumed {
                        binding_sha256: issued_binding,
                        review_id: review_id.to_owned(),
                        attestation_sha256: issued_attestation,
                    },
                    true,
                ))
            }
            _ => Ok((state, false)),
        })
    }

    fn mutate<T>(
        &mut self,
        update: impl FnOnce(
            PromotionLedgerState,
        ) -> Result<(PromotionLedgerState, T), PromotionLedgerError>,
    ) -> Result<T, PromotionLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        let repaired = pending.is_some() || current.partial_tail_from.is_some();
        if let Some(pending) = pending {
            remove_pending_publication(&self.root, &pending)?;
            sync_directory(&self.root).map_err(map_storage)?;
        }
        if let Some(length) = current.partial_tail_from {
            self.anchor
                .set_len(length)
                .map_err(|_| PromotionLedgerError::new("promotion-anchor-repair-failed"))?;
            self.anchor
                .sync_all()
                .map_err(|_| PromotionLedgerError::new("promotion-anchor-fsync-failed"))?;
        }
        if current.snapshot.head_sha256 != self.expected_head {
            self.expected_head = current.snapshot.head_sha256.clone();
        }
        let (next_state, result) = update(current.snapshot.payload.core.state.clone())?;
        if next_state == current.snapshot.payload.core.state && !repaired {
            test_final_validation_pause(&self.root_path);
            self.require_unchanged_current(None, &current)?;
            return Ok(result);
        }
        let current = current.snapshot;
        let generation = current
            .payload
            .core
            .generation
            .checked_add(1)
            .ok_or_else(|| PromotionLedgerError::new("promotion-ledger-generation-overflow"))?;
        let core = ReviewSnapshotCore {
            schema_version: "PromotionReviewLedger-v1".to_owned(),
            generation,
            previous_head_sha256: current.head_sha256,
            key_id: self.key_id.clone(),
            lock_identity: self.lock_identity,
            anchor_authority: self.anchor_authority,
            binding: self.binding.clone(),
            state: next_state,
        };
        let record = authenticate_anchor_record(
            ReviewAnchorRecordPayload {
                schema_version: "PromotionReviewAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: current.payload.anchor_head_sha256,
                core: core.clone(),
            },
            &self.key,
        )?;
        let (anchor_observation, anchor_length) = append_anchor_record(&self.anchor, &record)?;
        let next = authenticate_snapshot(
            ReviewSnapshotPayload {
                core,
                anchor_observation,
                anchor_length,
                anchor_head_sha256: record.head_sha256,
            },
            &self.key,
        )?;
        test_publication_pause(&self.root_path);
        publish_file(&self.root, STATE_NAME, &next, generation).map_err(map_storage)?;
        sync_directory(&self.root).map_err(map_storage)?;
        test_final_validation_pause(&self.root_path);
        self.require_published_current(&next)?;
        self.expected_head = next.head_sha256;
        Ok(result)
    }

    fn read_locked_current(
        &self,
    ) -> Result<(Option<PendingReviewPublication>, CurrentReviewSnapshot), PromotionLedgerError>
    {
        let pending = ensure_entries(&self.root_path, &self.root, false)?;
        let current = read_current(
            &self.root,
            &self.anchor,
            self.lock_identity,
            self.anchor_authority,
            &self.key,
            &self.key_id,
            &self.binding,
        )?;
        validate_pending_generation(pending.as_ref(), &current.snapshot)?;
        Ok((pending, current))
    }

    fn require_unchanged_current(
        &self,
        expected_pending: Option<&PendingReviewPublication>,
        expected: &CurrentReviewSnapshot,
    ) -> Result<(), PromotionLedgerError> {
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if pending.as_ref() != expected_pending
            || current.snapshot.head_sha256 != expected.snapshot.head_sha256
            || current.observed_state_head_sha256 != expected.observed_state_head_sha256
            || current.observed_anchor != expected.observed_anchor
            || current.partial_tail_from != expected.partial_tail_from
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-final-current-changed",
            ));
        }
        self.validate_descriptors()
    }

    fn require_published_current(
        &self,
        expected: &AuthenticatedReviewSnapshot,
    ) -> Result<(), PromotionLedgerError> {
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if pending.is_some()
            || current.partial_tail_from.is_some()
            || current.snapshot.head_sha256 != expected.head_sha256
            || current.observed_state_head_sha256 != expected.head_sha256
            || current.observed_anchor != expected.payload.anchor_observation
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-final-current-changed",
            ));
        }
        self.validate_descriptors()
    }

    fn validate_descriptors(&self) -> Result<(), PromotionLedgerError> {
        let root = safe_file_identity(&self.root)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-root-stat-failed"))?;
        let lock = safe_file_identity(&self.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-stat-failed"))?;
        let anchor = safe_file_identity(&self.anchor).map_err(map_storage)?;
        let named_root = open_safe_directory(&self.root_path)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-descriptor-substituted"))?
            .1;
        if root.device != self.root_identity.device
            || root.inode != self.root_identity.inode
            || root.mode != self.root_identity.mode
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-root-descriptor-substituted",
            ));
        }
        if lock != self.lock_identity {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-lock-descriptor-substituted",
            ));
        }
        if anchor.authority() != self.anchor_authority {
            return Err(PromotionLedgerError::new(
                "promotion-anchor-descriptor-substituted",
            ));
        }
        require_named_review_lock_identity(&self.root, self.lock_identity)?;
        require_named_review_anchor_authority(&self.root, self.anchor_authority)?;
        if named_root.device != root.device
            || named_root.inode != root.inode
            || named_root.mode != root.mode
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-descriptor-substituted",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set_test_publication_pause(root: PathBuf, milliseconds: u64) {
        publication_hooks()
            .lock()
            .expect("promotion publication hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_publication_is_paused(root: &Path) -> bool {
        publication_hooks()
            .lock()
            .expect("promotion publication hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_publication(root: &Path) {
        if let Some(hook) = publication_hooks()
            .lock()
            .expect("promotion publication hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_final_validation_pause(root: PathBuf, milliseconds: u64) {
        final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_final_validation_is_paused(root: &Path) -> bool {
        final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_final_validation(root: &Path) {
        if let Some(hook) = final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_directory_scan_pause(root: PathBuf, milliseconds: u64) {
        directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_directory_scan_is_paused(root: &Path) -> bool {
        directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_directory_scan(root: &Path) {
        if let Some(hook) = directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }
}

#[cfg(test)]
struct TestPause {
    milliseconds: u64,
    paused: bool,
    released: bool,
}

#[cfg(test)]
impl TestPause {
    fn new(milliseconds: u64) -> Self {
        Self {
            milliseconds,
            paused: false,
            released: false,
        }
    }
}

#[cfg(test)]
fn publication_hooks() -> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>
{
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

#[cfg(test)]
fn test_publication_pause(root: &Path) {
    let milliseconds = {
        let mut hooks = publication_hooks()
            .lock()
            .expect("promotion publication hook lock");
        let Some(hook) = hooks.get_mut(root) else {
            return;
        };
        hook.paused = true;
        hook.milliseconds
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(milliseconds);
    loop {
        let mut hooks = publication_hooks()
            .lock()
            .expect("promotion publication hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release promotion publication"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(not(test))]
fn test_publication_pause(_: &Path) {}

#[cfg(test)]
fn final_validation_hooks()
-> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

#[cfg(test)]
fn test_final_validation_pause(root: &Path) {
    let milliseconds = {
        let mut hooks = final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock");
        let Some(hook) = hooks.get_mut(root) else {
            return;
        };
        hook.paused = true;
        hook.milliseconds
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(milliseconds);
    loop {
        let mut hooks = final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release promotion final validation"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(not(test))]
fn test_final_validation_pause(_: &Path) {}

#[cfg(test)]
fn directory_scan_hooks()
-> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

#[cfg(test)]
fn test_directory_scan_pause(root: &Path) {
    let milliseconds = {
        let mut hooks = directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock");
        let Some(hook) = hooks.get_mut(root) else {
            return;
        };
        hook.paused = true;
        hook.milliseconds
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(milliseconds);
    loop {
        let mut hooks = directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release promotion directory scan"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(not(test))]
fn test_directory_scan_pause(_: &Path) {}

impl PromotionReviewAuthority for FilePromotionReviewLedger {
    fn authority_id(&self) -> &str {
        &self.binding.authority_id
    }

    fn reviewer_id(&self) -> &str {
        &self.binding.reviewer_id
    }

    fn review_session_id(&self) -> &str {
        &self.binding.review_session_id
    }

    fn current_binding(&self) -> (&str, &str) {
        (&self.binding.live_context_id, &self.binding.candidate_id)
    }

    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, EvaluationError> {
        self.issue_bound_attestation(binding_sha256)
            .map_err(|_| EvaluationError::new("evaluation-review-ledger-issuance-refused"))
    }

    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        reviewer_id: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> bool {
        self.consume_attestation(binding_sha256, reviewer_id, review_id, attestation_sha256)
            .unwrap_or(false)
    }
}

fn validate_binding(binding: &PromotionLedgerBinding) -> Result<(), PromotionLedgerError> {
    if binding.authority_id == binding.reviewer_id
        || binding.review_session_id == binding.baseline_execution_session_id
        || binding.review_session_id == binding.candidate_execution_session_id
        || binding.baseline_execution_session_id == binding.candidate_execution_session_id
        || !super::valid_identifier(&binding.authority_id)
        || !super::valid_identifier(&binding.reviewer_id)
        || [
            binding.review_session_id.as_str(),
            binding.live_context_id.as_str(),
            binding.baseline_candidate_id.as_str(),
            binding.candidate_id.as_str(),
            binding.baseline_run_sha256.as_str(),
            binding.candidate_run_sha256.as_str(),
            binding.baseline_execution_head_sha256.as_str(),
            binding.candidate_execution_head_sha256.as_str(),
        ]
        .iter()
        .any(|value| !super::valid_sha256(value))
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-binding-invalid",
        ));
    }
    Ok(())
}

fn authenticate_snapshot(
    payload: ReviewSnapshotPayload,
    key: &[u8; 32],
) -> Result<AuthenticatedReviewSnapshot, PromotionLedgerError> {
    let bytes = serde_json::to_vec(&payload)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-serialization-failed"))?;
    let mac_sha256 = hmac(&bytes, key).map_err(map_storage)?;
    let head_sha256 = sha256(format!("{}|{mac_sha256}", sha256(&bytes)).as_bytes());
    Ok(AuthenticatedReviewSnapshot {
        payload,
        mac_sha256,
        head_sha256,
    })
}

fn authenticate_anchor_record(
    payload: ReviewAnchorRecordPayload,
    key: &[u8; 32],
) -> Result<AuthenticatedReviewAnchorRecord, PromotionLedgerError> {
    let bytes = serde_json::to_vec(&payload)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-serialization-failed"))?;
    let mac_sha256 = hmac(&bytes, key).map_err(map_storage)?;
    let head_sha256 = sha256(format!("{}|{mac_sha256}", sha256(&bytes)).as_bytes());
    Ok(AuthenticatedReviewAnchorRecord {
        payload,
        mac_sha256,
        head_sha256,
    })
}

fn read_current(
    root: &File,
    anchor: &File,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    key: &[u8; 32],
    key_id: &str,
    binding: &PromotionLedgerBinding,
) -> Result<CurrentReviewSnapshot, PromotionLedgerError> {
    require_named_review_lock_identity(root, lock_identity)?;
    require_named_review_anchor_authority(root, anchor_authority)?;
    let snapshot: AuthenticatedReviewSnapshot =
        read_json_file(root, STATE_NAME).map_err(map_storage)?;
    let expected = authenticate_snapshot(snapshot.payload.clone(), key)?;
    if snapshot.payload.core.schema_version != "PromotionReviewLedger-v1"
        || snapshot.payload.core.key_id != key_id
        || snapshot.payload.core.lock_identity != lock_identity
        || snapshot.payload.core.anchor_authority != anchor_authority
        || snapshot.payload.core.binding != *binding
        || snapshot.payload.anchor_observation.authority() != anchor_authority
        || snapshot.payload.anchor_length != snapshot.payload.anchor_observation.length
        || !super::valid_sha256(&snapshot.payload.anchor_head_sha256)
        || snapshot.mac_sha256 != expected.mac_sha256
        || snapshot.head_sha256 != expected.head_sha256
        || !state_valid(&snapshot.payload.core.state)
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-authentication-failed",
        ));
    }
    let observed_state_head_sha256 = snapshot.head_sha256.clone();
    let observation_before = safe_file_identity(anchor).map_err(map_storage)?;
    if observation_before.authority() != anchor_authority {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-descriptor-substituted",
        ));
    }
    let bytes = read_anchor_bytes(anchor)?;
    let observation_after = safe_file_identity(anchor).map_err(map_storage)?;
    if observation_after != observation_before {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-changed-during-read",
        ));
    }
    let scan = scan_anchor_journal(
        &bytes,
        key,
        key_id,
        lock_identity,
        anchor_authority,
        binding,
    )?;
    let stored_index = scan.records.iter().position(|record| {
        record.end == snapshot.payload.anchor_length
            && record.record.head_sha256 == snapshot.payload.anchor_head_sha256
            && record.record.payload.core == snapshot.payload.core
    });
    let Some(stored_index) = stored_index else {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-state-binding-invalid",
        ));
    };
    let extra = &scan.records[stored_index + 1..];
    match extra {
        [] => {
            if scan.complete_length != snapshot.payload.anchor_length {
                return Err(PromotionLedgerError::new(
                    "promotion-anchor-journal-ambiguous",
                ));
            }
            if scan.partial_tail {
                if observation_after.length <= scan.complete_length {
                    return Err(PromotionLedgerError::new(
                        "promotion-anchor-partial-tail-invalid",
                    ));
                }
                Ok(CurrentReviewSnapshot {
                    snapshot,
                    partial_tail_from: Some(scan.complete_length),
                    observed_state_head_sha256,
                    observed_anchor: observation_after,
                })
            } else if observation_after == snapshot.payload.anchor_observation
                && observation_after.length == snapshot.payload.anchor_length
            {
                Ok(CurrentReviewSnapshot {
                    snapshot,
                    partial_tail_from: None,
                    observed_state_head_sha256,
                    observed_anchor: observation_after,
                })
            } else {
                Err(PromotionLedgerError::new(
                    "promotion-anchor-rollback-or-mutation-detected",
                ))
            }
        }
        [successor] => {
            if scan.partial_tail
                || successor.record.payload.core.generation
                    != snapshot.payload.core.generation.saturating_add(1)
                || successor.record.payload.core.previous_head_sha256 != snapshot.head_sha256
                || successor.record.payload.prior_anchor_head_sha256
                    != snapshot.payload.anchor_head_sha256
            {
                return Err(PromotionLedgerError::new(
                    "promotion-anchor-recovery-chain-invalid",
                ));
            }
            let recovered = authenticate_snapshot(
                ReviewSnapshotPayload {
                    core: successor.record.payload.core.clone(),
                    anchor_observation: observation_after,
                    anchor_length: successor.end,
                    anchor_head_sha256: successor.record.head_sha256.clone(),
                },
                key,
            )?;
            Ok(CurrentReviewSnapshot {
                snapshot: recovered,
                partial_tail_from: None,
                observed_state_head_sha256,
                observed_anchor: observation_after,
            })
        }
        _ => Err(PromotionLedgerError::new(
            "promotion-anchor-recovery-depth-exceeded",
        )),
    }
}

fn verify_anchor_record(
    record: &AuthenticatedReviewAnchorRecord,
    key: &[u8; 32],
) -> Result<(), PromotionLedgerError> {
    let expected = authenticate_anchor_record(record.payload.clone(), key)?;
    if record.mac_sha256 != expected.mac_sha256 || record.head_sha256 != expected.head_sha256 {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-authentication-failed",
        ));
    }
    Ok(())
}

struct ScannedReviewAnchorRecord {
    end: u64,
    record: AuthenticatedReviewAnchorRecord,
}

struct ReviewAnchorJournalScan {
    records: Vec<ScannedReviewAnchorRecord>,
    complete_length: u64,
    partial_tail: bool,
}

fn scan_anchor_journal(
    bytes: &[u8],
    key: &[u8; 32],
    key_id: &str,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &PromotionLedgerBinding,
) -> Result<ReviewAnchorJournalScan, PromotionLedgerError> {
    let mut records = Vec::new();
    let mut offset = 0_usize;
    let mut prior_head = sha256(ANCHOR_GENESIS);
    let mut generation = 0_u64;
    while offset < bytes.len() {
        if bytes.len() - offset < 8 {
            return Ok(ReviewAnchorJournalScan {
                records,
                complete_length: offset as u64,
                partial_tail: true,
            });
        }
        let length = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        if length == 0 || length > MAX_ANCHOR_RECORD_BYTES {
            return Err(PromotionLedgerError::new(
                "promotion-anchor-frame-length-invalid",
            ));
        }
        let end = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(length))
            .ok_or_else(|| PromotionLedgerError::new("promotion-anchor-frame-overflow"))?;
        if end > bytes.len() {
            return Ok(ReviewAnchorJournalScan {
                records,
                complete_length: offset as u64,
                partial_tail: true,
            });
        }
        let record: AuthenticatedReviewAnchorRecord =
            serde_json::from_slice(&bytes[offset + 8..end])
                .map_err(|_| PromotionLedgerError::new("promotion-anchor-record-malformed"))?;
        verify_anchor_record(&record, key)?;
        if record.payload.schema_version != "PromotionReviewAnchorRecord-v1"
            || record.payload.prior_anchor_head_sha256 != prior_head
            || record.payload.core.generation != generation
            || record.payload.core.key_id != key_id
            || record.payload.core.lock_identity != lock_identity
            || record.payload.core.anchor_authority != anchor_authority
            || record.payload.core.binding != *binding
            || !state_valid(&record.payload.core.state)
        {
            return Err(PromotionLedgerError::new(
                "promotion-anchor-record-binding-invalid",
            ));
        }
        prior_head = record.head_sha256.clone();
        generation = generation
            .checked_add(1)
            .ok_or_else(|| PromotionLedgerError::new("promotion-ledger-generation-overflow"))?;
        records.push(ScannedReviewAnchorRecord {
            end: end as u64,
            record,
        });
        offset = end;
    }
    Ok(ReviewAnchorJournalScan {
        records,
        complete_length: offset as u64,
        partial_tail: false,
    })
}

fn state_valid(state: &PromotionLedgerState) -> bool {
    match state {
        PromotionLedgerState::Ready => true,
        PromotionLedgerState::Issued {
            binding_sha256,
            attestation_sha256,
        } => super::valid_sha256(binding_sha256) && super::valid_sha256(attestation_sha256),
        PromotionLedgerState::Consumed {
            binding_sha256,
            review_id,
            attestation_sha256,
        } => {
            super::valid_sha256(binding_sha256)
                && super::valid_sha256(review_id)
                && super::valid_sha256(attestation_sha256)
        }
        PromotionLedgerState::RecoveryRequired { causal_code } => {
            super::valid_identifier(causal_code)
        }
    }
}

fn open_review_lock(root: &File, create: bool) -> Result<File, PromotionLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, LOCK_NAME, flags, 0o600).map_err(map_storage)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file).map_err(map_storage)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32 || identity.links != 1 {
        return Err(PromotionLedgerError::new("promotion-ledger-lock-unsafe"));
    }
    Ok(file)
}

fn open_review_anchor(root: &File, create: bool) -> Result<File, PromotionLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, ANCHOR_NAME, flags, 0o600).map_err(map_storage)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file).map_err(map_storage)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32 || identity.links != 1 {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-authority-unsafe",
        ));
    }
    Ok(file)
}

fn require_named_review_lock_identity(
    root: &File,
    expected: FileIdentity,
) -> Result<(), PromotionLedgerError> {
    let current = open_review_lock(root, false)?;
    if safe_file_identity(&current).map_err(map_storage)? != expected {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-lock-descriptor-substituted",
        ));
    }
    Ok(())
}

fn require_named_review_anchor_authority(
    root: &File,
    expected: FileAuthorityIdentity,
) -> Result<(), PromotionLedgerError> {
    let current = open_review_anchor(root, false)?;
    if safe_file_identity(&current)
        .map_err(map_storage)?
        .authority()
        != expected
    {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-descriptor-substituted",
        ));
    }
    Ok(())
}

fn read_anchor_bytes(anchor: &File) -> Result<Vec<u8>, PromotionLedgerError> {
    use std::os::unix::fs::FileExt;

    let identity = safe_file_identity(anchor).map_err(map_storage)?;
    if identity.length == 0 || identity.length > MAX_ANCHOR_JOURNAL_BYTES {
        return Err(PromotionLedgerError::new("promotion-anchor-length-invalid"));
    }
    let mut bytes = vec![0_u8; identity.length as usize];
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let read = anchor
            .read_at(&mut bytes[offset..], offset as u64)
            .map_err(|_| PromotionLedgerError::new("promotion-anchor-read-failed"))?;
        if read == 0 {
            return Err(PromotionLedgerError::new("promotion-anchor-read-truncated"));
        }
        offset += read;
    }
    Ok(bytes)
}

fn append_anchor_record(
    anchor: &File,
    record: &AuthenticatedReviewAnchorRecord,
) -> Result<(FileIdentity, u64), PromotionLedgerError> {
    let bytes = serde_json::to_vec(record)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-serialization-failed"))?;
    if bytes.is_empty() || bytes.len() > MAX_ANCHOR_RECORD_BYTES {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-record-length-invalid",
        ));
    }
    let before = safe_file_identity(anchor).map_err(map_storage)?;
    let frame_length = 8_u64
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| PromotionLedgerError::new("promotion-anchor-frame-overflow"))?;
    let end = before
        .length
        .checked_add(frame_length)
        .filter(|end| *end <= MAX_ANCHOR_JOURNAL_BYTES)
        .ok_or_else(|| PromotionLedgerError::new("promotion-anchor-journal-full"))?;
    let header = (bytes.len() as u64).to_be_bytes();
    write_all_at(anchor, &header, before.length)?;
    write_all_at(anchor, &bytes, before.length + header.len() as u64)?;
    anchor
        .sync_all()
        .map_err(|_| PromotionLedgerError::new("promotion-anchor-fsync-failed"))?;
    let after = safe_file_identity(anchor).map_err(map_storage)?;
    if after.authority() != before.authority() || after.length != end {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-changed-during-append",
        ));
    }
    Ok((after, end))
}

fn write_all_at(anchor: &File, bytes: &[u8], offset: u64) -> Result<(), PromotionLedgerError> {
    use std::os::unix::fs::FileExt;

    let mut written = 0_usize;
    while written < bytes.len() {
        let count = anchor
            .write_at(&bytes[written..], offset + written as u64)
            .map_err(|_| PromotionLedgerError::new("promotion-anchor-write-failed"))?;
        if count == 0 {
            return Err(PromotionLedgerError::new("promotion-anchor-write-failed"));
        }
        written += count;
    }
    Ok(())
}

fn ensure_entries(
    path: &Path,
    root: &File,
    initializing: bool,
) -> Result<Option<PendingReviewPublication>, PromotionLedgerError> {
    let allowed = [ANCHOR_NAME, LOCK_NAME, STATE_NAME];
    test_directory_scan_pause(path);
    let entries = read_directory_names(root).map_err(map_storage)?;
    let mut pending = None;
    for name in entries {
        if !initializing
            && allowed
                .iter()
                .any(|allowed| name == std::ffi::OsStr::new(allowed))
        {
            continue;
        }
        let Some(name) = name.to_str() else {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-unknown-or-pending-entry",
            ));
        };
        let Some(generation) = (!initializing)
            .then(|| pending_generation(name, STATE_NAME))
            .flatten()
        else {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-unknown-or-pending-entry",
            ));
        };
        if pending.is_some() {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-multiple-pending-publications",
            ));
        }
        let descriptor = openat(
            root,
            name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )
        .map_err(map_storage)?;
        let file = unsafe { File::from_raw_fd(descriptor) };
        let identity = safe_file_identity(&file).map_err(map_storage)?;
        if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32
            || identity.links != 1
            || identity.mode & 0o777 != 0o600
            || identity.length > MAX_LEDGER_BYTES
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-pending-publication-unsafe",
            ));
        }
        pending = Some(PendingReviewPublication {
            name: name.to_owned(),
            generation,
            identity,
        });
    }
    Ok(pending)
}

fn pending_generation(name: &str, destination: &str) -> Option<u64> {
    let remainder = name.strip_prefix(&format!(".{destination}.pending."))?;
    let (pid, generation) = remainder.split_once('.')?;
    if pid.is_empty()
        || generation.is_empty()
        || generation.contains('.')
        || !pid.bytes().all(|byte| byte.is_ascii_digit())
        || !generation.bytes().all(|byte| byte.is_ascii_digit())
        || (pid.len() > 1 && pid.starts_with('0'))
        || (generation.len() > 1 && generation.starts_with('0'))
        || pid.parse::<u32>().ok()? == 0
    {
        return None;
    }
    generation.parse().ok()
}

fn validate_pending_generation(
    pending: Option<&PendingReviewPublication>,
    snapshot: &AuthenticatedReviewSnapshot,
) -> Result<(), PromotionLedgerError> {
    if pending.is_some_and(|pending| pending.generation != snapshot.payload.core.generation) {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-pending-generation-invalid",
        ));
    }
    Ok(())
}

fn remove_pending_publication(
    root: &File,
    pending: &PendingReviewPublication,
) -> Result<(), PromotionLedgerError> {
    let descriptor = openat(
        root,
        &pending.name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        0,
    )
    .map_err(map_storage)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    if safe_file_identity(&file).map_err(map_storage)? != pending.identity {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-pending-publication-substituted",
        ));
    }
    let name = std::ffi::CString::new(pending.name.as_str())
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-component-invalid"))?;
    if unsafe { libc::unlinkat(root.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-pending-publication-remove-failed",
        ));
    }
    Ok(())
}

fn map_storage(_: super::EvaluationLedgerError) -> PromotionLedgerError {
    PromotionLedgerError::new("promotion-ledger-storage-failed")
}
