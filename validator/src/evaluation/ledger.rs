use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::{CStr, CString, OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

const STATE_NAME: &str = "execution.state";
const ANCHOR_NAME: &str = "execution.anchor.journal";
const LOCK_NAME: &str = "execution.lock";
const MAX_LEDGER_BYTES: u64 = 1024 * 1024;
const MAX_ANCHOR_JOURNAL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ANCHOR_RECORD_BYTES: usize = 1024 * 1024;
const ANCHOR_GENESIS: &[u8] = b"evaluation-anchor-journal-genesis";

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvaluationExecutionBinding {
    pub live_context_id: String,
    pub candidate_id: String,
    pub spec_sha256: String,
    pub task_set_sha256: String,
    pub execution_session_id: String,
    pub executable_set_sha256: String,
    pub artifact_root_sha256: String,
}

impl EvaluationExecutionBinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        live_context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        spec_sha256: impl Into<String>,
        task_set_sha256: impl Into<String>,
        execution_session_id: impl Into<String>,
        executable_set_sha256: impl Into<String>,
        artifact_root_sha256: impl Into<String>,
    ) -> Result<Self, EvaluationLedgerError> {
        let value = Self {
            live_context_id: live_context_id.into(),
            candidate_id: candidate_id.into(),
            spec_sha256: spec_sha256.into(),
            task_set_sha256: task_set_sha256.into(),
            execution_session_id: execution_session_id.into(),
            executable_set_sha256: executable_set_sha256.into(),
            artifact_root_sha256: artifact_root_sha256.into(),
        };
        if [
            value.live_context_id.as_str(),
            value.candidate_id.as_str(),
            value.spec_sha256.as_str(),
            value.task_set_sha256.as_str(),
            value.execution_session_id.as_str(),
            value.executable_set_sha256.as_str(),
            value.artifact_root_sha256.as_str(),
        ]
        .iter()
        .any(|item| !super::valid_sha256(item))
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-binding-invalid",
            ));
        }
        Ok(value)
    }

    pub(crate) fn digest(&self) -> String {
        sha256(&serde_json::to_vec(self).expect("execution binding serializes"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EvaluationLedgerState {
    Initialized,
    Reserved,
    Published {
        run_sha256: String,
        artifact_set_sha256: String,
    },
    Interrupted {
        causal_code: String,
    },
    RecoveryRequired {
        causal_code: String,
    },
    Terminal {
        run_sha256: String,
        artifact_set_sha256: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationLedgerError {
    code: &'static str,
}

impl EvaluationLedgerError {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for EvaluationLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for EvaluationLedgerError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExecutionTerminalProof {
    pub binding: EvaluationExecutionBinding,
    pub run_sha256: String,
    pub artifact_set_sha256: String,
    pub ledger_head_sha256: String,
}

#[derive(Debug)]
pub struct FileEvaluationExecutionLedger {
    root_path: PathBuf,
    root: File,
    root_identity: FileIdentity,
    lock: File,
    lock_identity: FileIdentity,
    anchor: File,
    anchor_authority: FileAuthorityIdentity,
    key: [u8; 32],
    key_id: String,
    binding: EvaluationExecutionBinding,
    expected_head: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct FileIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) mode: u32,
    pub(super) links: u64,
    pub(super) length: u64,
    pub(super) changed_seconds: i64,
    pub(super) changed_nanos: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct FileAuthorityIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) mode: u32,
    pub(super) links: u64,
}

impl FileIdentity {
    pub(super) const fn authority(self) -> FileAuthorityIdentity {
        FileAuthorityIdentity {
            device: self.device,
            inode: self.inode,
            mode: self.mode,
            links: self.links,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SnapshotCore {
    schema_version: String,
    generation: u64,
    previous_head_sha256: String,
    key_id: String,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: EvaluationExecutionBinding,
    state: EvaluationLedgerState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SnapshotPayload {
    core: SnapshotCore,
    anchor_observation: FileIdentity,
    anchor_length: u64,
    anchor_head_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatedSnapshot {
    payload: SnapshotPayload,
    mac_sha256: String,
    head_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AnchorRecordPayload {
    schema_version: String,
    prior_anchor_head_sha256: String,
    core: SnapshotCore,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatedAnchorRecord {
    payload: AnchorRecordPayload,
    mac_sha256: String,
    head_sha256: String,
}

struct CurrentSnapshot {
    snapshot: AuthenticatedSnapshot,
    partial_tail_from: Option<u64>,
    observed_state_head_sha256: String,
    observed_anchor: FileIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingPublication {
    name: String,
    generation: u64,
    identity: FileIdentity,
}

impl FileEvaluationExecutionLedger {
    pub fn initialize(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: EvaluationExecutionBinding,
    ) -> Result<Self, EvaluationLedgerError> {
        let root_path = root.as_ref().to_path_buf();
        let (root, root_identity) = open_safe_directory(&root_path)?;
        ensure_only_known_entries(&root_path, &root, true)?;
        let lock = open_lock(&root, true)?;
        let lock_identity = safe_file_identity(&lock)?;
        let anchor = open_anchor(&root, true)?;
        let anchor_authority = safe_file_identity(&anchor)?.authority();
        let key_id = sha256(&key);
        let core = SnapshotCore {
            schema_version: "EvaluationExecutionLedger-v1".to_owned(),
            generation: 0,
            previous_head_sha256: sha256(b"evaluation-ledger-genesis"),
            key_id: key_id.clone(),
            lock_identity,
            anchor_authority,
            binding: binding.clone(),
            state: EvaluationLedgerState::Initialized,
        };
        let record = authenticate_anchor_record(
            AnchorRecordPayload {
                schema_version: "EvaluationExecutionAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: sha256(ANCHOR_GENESIS),
                core: core.clone(),
            },
            &key,
        )?;
        require_named_lock_identity(&root, lock_identity)?;
        require_named_anchor_authority(&root, anchor_authority)?;
        let _guard = FileLock::exclusive(&lock)?;
        require_named_lock_identity(&root, lock_identity)?;
        require_named_anchor_authority(&root, anchor_authority)?;
        if entry_exists(&root, STATE_NAME)? {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-already-exists",
            ));
        }
        let (anchor_observation, anchor_length) = append_anchor_record(&anchor, &record)?;
        let snapshot = authenticate_snapshot(
            SnapshotPayload {
                core,
                anchor_observation,
                anchor_length,
                anchor_head_sha256: record.head_sha256,
            },
            &key,
        )?;
        publish_file(&root, STATE_NAME, &snapshot, 0)?;
        sync_directory(&root)?;
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
        let _guard = FileLock::exclusive(&ledger.lock)?;
        ledger.require_published_current(&snapshot)?;
        drop(_guard);
        Ok(ledger)
    }

    pub fn open(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: EvaluationExecutionBinding,
    ) -> Result<Self, EvaluationLedgerError> {
        let root_path = root.as_ref().to_path_buf();
        let (root, root_identity) = open_safe_directory(&root_path)?;
        let lock = open_lock(&root, false)?;
        let lock_identity = safe_file_identity(&lock)?;
        let anchor = open_anchor(&root, false)?;
        let anchor_authority = safe_file_identity(&anchor)?.authority();
        let key_id = sha256(&key);
        require_named_lock_identity(&root, lock_identity)?;
        require_named_anchor_authority(&root, anchor_authority)?;
        let (pending, current) = {
            let _guard = FileLock::exclusive(&lock)?;
            require_named_lock_identity(&root, lock_identity)?;
            require_named_anchor_authority(&root, anchor_authority)?;
            let pending = ensure_only_known_entries(&root_path, &root, false)?;
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
        let _guard = FileLock::exclusive(&ledger.lock)?;
        ledger.require_unchanged_current(pending.as_ref(), &current)?;
        drop(_guard);
        Ok(ledger)
    }

    /// Read-only authenticated inspection. This does not create lock, temp,
    /// recovery, or audit files.
    pub fn inspect(&self) -> Result<EvaluationLedgerState, EvaluationLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        let state = current.snapshot.payload.core.state.clone();
        test_final_validation_pause(&self.root_path);
        self.require_unchanged_current(pending.as_ref(), &current)?;
        Ok(state)
    }

    pub fn reserve(&mut self) -> Result<(), EvaluationLedgerError> {
        self.transition(|state| match state {
            EvaluationLedgerState::Initialized => Ok(EvaluationLedgerState::Reserved),
            _ => Err(EvaluationLedgerError::new(
                "evaluation-execution-reservation-conflict",
            )),
        })
    }

    pub fn publish_result(
        &mut self,
        run_sha256: impl Into<String>,
        artifact_set_sha256: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        let run_sha256 = run_sha256.into();
        let artifact_set_sha256 = artifact_set_sha256.into();
        if !super::valid_sha256(&run_sha256) || !super::valid_sha256(&artifact_set_sha256) {
            return Err(EvaluationLedgerError::new(
                "evaluation-publication-binding-invalid",
            ));
        }
        self.transition(|state| match state {
            EvaluationLedgerState::Reserved => Ok(EvaluationLedgerState::Published {
                run_sha256,
                artifact_set_sha256,
            }),
            _ => Err(EvaluationLedgerError::new(
                "evaluation-publication-transition-refused",
            )),
        })
    }

    pub fn mark_interrupted(
        &mut self,
        causal_code: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        let causal_code = checked_causal_code(causal_code)?;
        self.transition(|state| match state {
            EvaluationLedgerState::Reserved | EvaluationLedgerState::Published { .. } => {
                Ok(EvaluationLedgerState::Interrupted { causal_code })
            }
            _ => Err(EvaluationLedgerError::new(
                "evaluation-interruption-transition-refused",
            )),
        })
    }

    pub fn require_recovery(
        &mut self,
        causal_code: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        let causal_code = checked_causal_code(causal_code)?;
        self.transition(|state| match state {
            EvaluationLedgerState::Reserved
            | EvaluationLedgerState::Published { .. }
            | EvaluationLedgerState::Interrupted { .. } => {
                Ok(EvaluationLedgerState::RecoveryRequired { causal_code })
            }
            _ => Err(EvaluationLedgerError::new(
                "evaluation-recovery-transition-refused",
            )),
        })
    }

    pub fn reconcile_recovery(
        &mut self,
        recovered_publication: Option<(String, String)>,
    ) -> Result<(), EvaluationLedgerError> {
        if let Some((run, artifacts)) = &recovered_publication
            && (!super::valid_sha256(run) || !super::valid_sha256(artifacts))
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-recovery-binding-invalid",
            ));
        }
        self.transition(|state| match state {
            EvaluationLedgerState::RecoveryRequired { .. } => match recovered_publication {
                Some((run_sha256, artifact_set_sha256)) => Ok(EvaluationLedgerState::Published {
                    run_sha256,
                    artifact_set_sha256,
                }),
                None => Ok(EvaluationLedgerState::Interrupted {
                    causal_code: "recovery-confirmed-no-publication".to_owned(),
                }),
            },
            _ => Err(EvaluationLedgerError::new(
                "evaluation-recovery-not-required",
            )),
        })
    }

    pub fn complete(&mut self) -> Result<(), EvaluationLedgerError> {
        self.transition(|state| match state {
            EvaluationLedgerState::Published {
                run_sha256,
                artifact_set_sha256,
            } => Ok(EvaluationLedgerState::Terminal {
                run_sha256,
                artifact_set_sha256,
            }),
            _ => Err(EvaluationLedgerError::new(
                "evaluation-terminal-transition-refused",
            )),
        })
    }

    pub(crate) fn terminal_proof(&self) -> Result<ExecutionTerminalProof, EvaluationLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        test_final_validation_pause(&self.root_path);
        self.require_unchanged_current(pending.as_ref(), &current)?;
        let snapshot = current.snapshot;
        let EvaluationLedgerState::Terminal {
            run_sha256,
            artifact_set_sha256,
        } = snapshot.payload.core.state
        else {
            return Err(EvaluationLedgerError::new(
                "evaluation-terminal-proof-unavailable",
            ));
        };
        Ok(ExecutionTerminalProof {
            binding: self.binding.clone(),
            run_sha256,
            artifact_set_sha256,
            ledger_head_sha256: snapshot.head_sha256,
        })
    }

    fn transition(
        &mut self,
        update: impl FnOnce(
            EvaluationLedgerState,
        ) -> Result<EvaluationLedgerState, EvaluationLedgerError>,
    ) -> Result<(), EvaluationLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if let Some(pending) = pending {
            remove_pending_publication(&self.root, &pending)?;
            sync_directory(&self.root)?;
        }
        if let Some(length) = current.partial_tail_from {
            self.anchor
                .set_len(length)
                .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-repair-failed"))?;
            self.anchor
                .sync_all()
                .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-fsync-failed"))?;
        }
        let current = current.snapshot;
        if current.head_sha256 != self.expected_head {
            self.expected_head = current.head_sha256.clone();
        }
        let next_state = update(current.payload.core.state.clone())?;
        let next_generation = current
            .payload
            .core
            .generation
            .checked_add(1)
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-ledger-generation-overflow"))?;
        let core = SnapshotCore {
            schema_version: "EvaluationExecutionLedger-v1".to_owned(),
            generation: next_generation,
            previous_head_sha256: current.head_sha256,
            key_id: self.key_id.clone(),
            lock_identity: self.lock_identity,
            anchor_authority: self.anchor_authority,
            binding: self.binding.clone(),
            state: next_state,
        };
        let record = authenticate_anchor_record(
            AnchorRecordPayload {
                schema_version: "EvaluationExecutionAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: current.payload.anchor_head_sha256,
                core: core.clone(),
            },
            &self.key,
        )?;
        let (anchor_observation, anchor_length) = append_anchor_record(&self.anchor, &record)?;
        let next = authenticate_snapshot(
            SnapshotPayload {
                core,
                anchor_observation,
                anchor_length,
                anchor_head_sha256: record.head_sha256,
            },
            &self.key,
        )?;
        test_publication_pause(&self.root_path);
        publish_file(&self.root, STATE_NAME, &next, next_generation)?;
        sync_directory(&self.root)?;
        test_final_validation_pause(&self.root_path);
        self.require_published_current(&next)?;
        self.expected_head = next.head_sha256;
        Ok(())
    }

    fn read_locked_current(
        &self,
    ) -> Result<(Option<PendingPublication>, CurrentSnapshot), EvaluationLedgerError> {
        let pending = ensure_only_known_entries(&self.root_path, &self.root, false)?;
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
        expected_pending: Option<&PendingPublication>,
        expected: &CurrentSnapshot,
    ) -> Result<(), EvaluationLedgerError> {
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if pending.as_ref() != expected_pending
            || current.snapshot.head_sha256 != expected.snapshot.head_sha256
            || current.observed_state_head_sha256 != expected.observed_state_head_sha256
            || current.observed_anchor != expected.observed_anchor
            || current.partial_tail_from != expected.partial_tail_from
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-final-current-changed",
            ));
        }
        self.validate_descriptors()
    }

    fn require_published_current(
        &self,
        expected: &AuthenticatedSnapshot,
    ) -> Result<(), EvaluationLedgerError> {
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if pending.is_some()
            || current.partial_tail_from.is_some()
            || current.snapshot.head_sha256 != expected.head_sha256
            || current.observed_state_head_sha256 != expected.head_sha256
            || current.observed_anchor != expected.payload.anchor_observation
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-final-current-changed",
            ));
        }
        self.validate_descriptors()
    }

    fn validate_descriptors(&self) -> Result<(), EvaluationLedgerError> {
        let root = safe_file_identity(&self.root)?;
        let lock = safe_file_identity(&self.lock)?;
        let anchor = safe_file_identity(&self.anchor)?;
        let named_root = open_safe_directory(&self.root_path)
            .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-descriptor-substituted"))?
            .1;
        if root.device != self.root_identity.device
            || root.inode != self.root_identity.inode
            || root.mode != self.root_identity.mode
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-root-descriptor-substituted",
            ));
        }
        if lock != self.lock_identity {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-lock-descriptor-substituted",
            ));
        }
        if anchor.authority() != self.anchor_authority {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-descriptor-substituted",
            ));
        }
        require_named_lock_identity(&self.root, self.lock_identity)?;
        require_named_anchor_authority(&self.root, self.anchor_authority)?;
        if named_root.device != root.device
            || named_root.inode != root.inode
            || named_root.mode != root.mode
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-descriptor-substituted",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set_test_publication_pause(root: PathBuf, milliseconds: u64) {
        publication_hooks()
            .lock()
            .expect("evaluation publication hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_publication_is_paused(root: &Path) -> bool {
        publication_hooks()
            .lock()
            .expect("evaluation publication hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_publication(root: &Path) {
        if let Some(hook) = publication_hooks()
            .lock()
            .expect("evaluation publication hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_final_validation_pause(root: PathBuf, milliseconds: u64) {
        final_validation_hooks()
            .lock()
            .expect("evaluation final validation hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_final_validation_is_paused(root: &Path) -> bool {
        final_validation_hooks()
            .lock()
            .expect("evaluation final validation hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_final_validation(root: &Path) {
        if let Some(hook) = final_validation_hooks()
            .lock()
            .expect("evaluation final validation hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_directory_scan_pause(root: PathBuf, milliseconds: u64) {
        directory_scan_hooks()
            .lock()
            .expect("evaluation directory scan hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_directory_scan_is_paused(root: &Path) -> bool {
        directory_scan_hooks()
            .lock()
            .expect("evaluation directory scan hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_directory_scan(root: &Path) {
        if let Some(hook) = directory_scan_hooks()
            .lock()
            .expect("evaluation directory scan hook lock")
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
            .expect("evaluation publication hook lock");
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
            .expect("evaluation publication hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release evaluation publication"
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
            .expect("evaluation final validation hook lock");
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
            .expect("evaluation final validation hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release evaluation final validation"
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
            .expect("evaluation directory scan hook lock");
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
            .expect("evaluation directory scan hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release evaluation directory scan"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(not(test))]
fn test_directory_scan_pause(_: &Path) {}

fn checked_causal_code(value: impl Into<String>) -> Result<String, EvaluationLedgerError> {
    let value = value.into();
    if !super::valid_identifier(&value) {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-causal-code-invalid",
        ));
    }
    Ok(value)
}

fn read_current(
    root: &File,
    anchor: &File,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    key: &[u8; 32],
    key_id: &str,
    binding: &EvaluationExecutionBinding,
) -> Result<CurrentSnapshot, EvaluationLedgerError> {
    require_named_lock_identity(root, lock_identity)?;
    require_named_anchor_authority(root, anchor_authority)?;
    let snapshot: AuthenticatedSnapshot = read_json_file(root, STATE_NAME)?;
    verify_snapshot(
        &snapshot,
        key,
        key_id,
        lock_identity,
        anchor_authority,
        binding,
    )?;
    let observed_state_head_sha256 = snapshot.head_sha256.clone();
    let observation_before = safe_file_identity(anchor)?;
    if observation_before.authority() != anchor_authority {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-descriptor-substituted",
        ));
    }
    let bytes = read_file_bytes(anchor, MAX_ANCHOR_JOURNAL_BYTES)?;
    let observation_after = safe_file_identity(anchor)?;
    if observation_after != observation_before {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-changed-during-read",
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
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-state-binding-invalid",
        ));
    };
    let extra = &scan.records[stored_index + 1..];
    match extra {
        [] => {
            if scan.complete_length != snapshot.payload.anchor_length {
                return Err(EvaluationLedgerError::new(
                    "evaluation-anchor-journal-ambiguous",
                ));
            }
            if scan.partial_tail {
                if observation_after.length <= scan.complete_length {
                    return Err(EvaluationLedgerError::new(
                        "evaluation-anchor-partial-tail-invalid",
                    ));
                }
                Ok(CurrentSnapshot {
                    snapshot,
                    partial_tail_from: Some(scan.complete_length),
                    observed_state_head_sha256,
                    observed_anchor: observation_after,
                })
            } else if observation_after == snapshot.payload.anchor_observation
                && observation_after.length == snapshot.payload.anchor_length
            {
                Ok(CurrentSnapshot {
                    snapshot,
                    partial_tail_from: None,
                    observed_state_head_sha256,
                    observed_anchor: observation_after,
                })
            } else {
                Err(EvaluationLedgerError::new(
                    "evaluation-anchor-rollback-or-mutation-detected",
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
                return Err(EvaluationLedgerError::new(
                    "evaluation-anchor-recovery-chain-invalid",
                ));
            }
            let recovered = authenticate_snapshot(
                SnapshotPayload {
                    core: successor.record.payload.core.clone(),
                    anchor_observation: observation_after,
                    anchor_length: successor.end,
                    anchor_head_sha256: successor.record.head_sha256.clone(),
                },
                key,
            )?;
            Ok(CurrentSnapshot {
                snapshot: recovered,
                partial_tail_from: None,
                observed_state_head_sha256,
                observed_anchor: observation_after,
            })
        }
        _ => Err(EvaluationLedgerError::new(
            "evaluation-anchor-recovery-depth-exceeded",
        )),
    }
}

fn authenticate_snapshot(
    payload: SnapshotPayload,
    key: &[u8; 32],
) -> Result<AuthenticatedSnapshot, EvaluationLedgerError> {
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    let mac_sha256 = hmac(&payload_bytes, key)?;
    let head_sha256 = sha256(format!("{}|{mac_sha256}", sha256(&payload_bytes)).as_bytes());
    Ok(AuthenticatedSnapshot {
        payload,
        mac_sha256,
        head_sha256,
    })
}

fn verify_snapshot(
    snapshot: &AuthenticatedSnapshot,
    key: &[u8; 32],
    key_id: &str,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &EvaluationExecutionBinding,
) -> Result<(), EvaluationLedgerError> {
    let expected = authenticate_snapshot(snapshot.payload.clone(), key)?;
    if snapshot.payload.core.schema_version != "EvaluationExecutionLedger-v1"
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
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-authentication-failed",
        ));
    }
    Ok(())
}

fn authenticate_anchor_record(
    payload: AnchorRecordPayload,
    key: &[u8; 32],
) -> Result<AuthenticatedAnchorRecord, EvaluationLedgerError> {
    let bytes = serde_json::to_vec(&payload)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    let mac_sha256 = hmac(&bytes, key)?;
    let head_sha256 = sha256(format!("{}|{mac_sha256}", sha256(&bytes)).as_bytes());
    Ok(AuthenticatedAnchorRecord {
        payload,
        mac_sha256,
        head_sha256,
    })
}

fn verify_anchor_record(
    record: &AuthenticatedAnchorRecord,
    key: &[u8; 32],
) -> Result<(), EvaluationLedgerError> {
    let expected = authenticate_anchor_record(record.payload.clone(), key)?;
    if record.mac_sha256 != expected.mac_sha256 || record.head_sha256 != expected.head_sha256 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-anchor-authentication-failed",
        ));
    }
    Ok(())
}

struct ScannedAnchorRecord {
    end: u64,
    record: AuthenticatedAnchorRecord,
}

struct AnchorJournalScan {
    records: Vec<ScannedAnchorRecord>,
    complete_length: u64,
    partial_tail: bool,
}

fn scan_anchor_journal(
    bytes: &[u8],
    key: &[u8; 32],
    key_id: &str,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &EvaluationExecutionBinding,
) -> Result<AnchorJournalScan, EvaluationLedgerError> {
    let mut records = Vec::new();
    let mut offset = 0_usize;
    let mut prior_head = sha256(ANCHOR_GENESIS);
    let mut generation = 0_u64;
    while offset < bytes.len() {
        if bytes.len() - offset < 8 {
            return Ok(AnchorJournalScan {
                records,
                complete_length: offset as u64,
                partial_tail: true,
            });
        }
        let length = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        if length == 0 || length > MAX_ANCHOR_RECORD_BYTES {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-frame-length-invalid",
            ));
        }
        let end = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(length))
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-anchor-frame-overflow"))?;
        if end > bytes.len() {
            return Ok(AnchorJournalScan {
                records,
                complete_length: offset as u64,
                partial_tail: true,
            });
        }
        let record: AuthenticatedAnchorRecord = serde_json::from_slice(&bytes[offset + 8..end])
            .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-record-malformed"))?;
        verify_anchor_record(&record, key)?;
        if record.payload.schema_version != "EvaluationExecutionAnchorRecord-v1"
            || record.payload.prior_anchor_head_sha256 != prior_head
            || record.payload.core.generation != generation
            || record.payload.core.key_id != key_id
            || record.payload.core.lock_identity != lock_identity
            || record.payload.core.anchor_authority != anchor_authority
            || record.payload.core.binding != *binding
            || !state_valid(&record.payload.core.state)
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-record-binding-invalid",
            ));
        }
        prior_head = record.head_sha256.clone();
        generation = generation
            .checked_add(1)
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-ledger-generation-overflow"))?;
        records.push(ScannedAnchorRecord {
            end: end as u64,
            record,
        });
        offset = end;
    }
    Ok(AnchorJournalScan {
        records,
        complete_length: offset as u64,
        partial_tail: false,
    })
}

fn state_valid(state: &EvaluationLedgerState) -> bool {
    match state {
        EvaluationLedgerState::Published {
            run_sha256,
            artifact_set_sha256,
        }
        | EvaluationLedgerState::Terminal {
            run_sha256,
            artifact_set_sha256,
        } => super::valid_sha256(run_sha256) && super::valid_sha256(artifact_set_sha256),
        EvaluationLedgerState::Interrupted { causal_code }
        | EvaluationLedgerState::RecoveryRequired { causal_code } => {
            super::valid_identifier(causal_code)
        }
        EvaluationLedgerState::Initialized | EvaluationLedgerState::Reserved => true,
    }
}

pub(super) fn open_safe_directory(
    path: &Path,
) -> Result<(File, FileIdentity), EvaluationLedgerError> {
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(EvaluationLedgerError::new("evaluation-ledger-root-unsafe"));
    }
    let root_name = CString::new("/").expect("root path has no NUL");
    let descriptor = unsafe {
        libc::open(
            root_name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-root-open-failed",
        ));
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    for component in components {
        let Component::Normal(component) = component else {
            return Err(EvaluationLedgerError::new("evaluation-ledger-root-unsafe"));
        };
        let name = CString::new(component.as_bytes())
            .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-root-unsafe"))?;
        let descriptor = unsafe {
            libc::openat(
                file.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0,
            )
        };
        if descriptor < 0 {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-root-open-failed",
            ));
        }
        file = unsafe { File::from_raw_fd(descriptor) };
    }
    let identity = safe_file_identity(&file)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFDIR as u32 || identity.mode & 0o022 != 0 {
        return Err(EvaluationLedgerError::new("evaluation-ledger-root-unsafe"));
    }
    Ok((file, identity))
}

struct DirectoryStream(*mut libc::DIR);

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn directory_errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
unsafe fn directory_errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

pub(super) fn read_directory_names(root: &File) -> Result<Vec<OsString>, EvaluationLedgerError> {
    let before = safe_file_identity(root)?;
    let descriptor = openat(
        root,
        ".",
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        0,
    )?;
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe {
            libc::close(descriptor);
        }
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-directory-read-failed",
        ));
    }
    let stream = DirectoryStream(stream);
    let mut names = Vec::new();
    loop {
        let errno = unsafe { directory_errno_location() };
        unsafe {
            *errno = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if unsafe { *errno } != 0 {
                return Err(EvaluationLedgerError::new(
                    "evaluation-ledger-directory-read-failed",
                ));
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        names.push(OsString::from_vec(name.to_vec()));
    }
    drop(stream);
    if safe_file_identity(root)? != before {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-directory-changed-during-read",
        ));
    }
    names.sort();
    Ok(names)
}

fn open_lock(root: &File, create: bool) -> Result<File, EvaluationLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, LOCK_NAME, flags, 0o600)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32 || identity.links != 1 {
        return Err(EvaluationLedgerError::new("evaluation-ledger-lock-unsafe"));
    }
    Ok(file)
}

fn open_anchor(root: &File, create: bool) -> Result<File, EvaluationLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, ANCHOR_NAME, flags, 0o600)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32 || identity.links != 1 {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-authority-unsafe",
        ));
    }
    Ok(file)
}

fn require_named_lock_identity(
    root: &File,
    expected: FileIdentity,
) -> Result<(), EvaluationLedgerError> {
    let current = open_lock(root, false)?;
    if safe_file_identity(&current)? != expected {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-lock-descriptor-substituted",
        ));
    }
    Ok(())
}

fn require_named_anchor_authority(
    root: &File,
    expected: FileAuthorityIdentity,
) -> Result<(), EvaluationLedgerError> {
    let current = open_anchor(root, false)?;
    if safe_file_identity(&current)?.authority() != expected {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-descriptor-substituted",
        ));
    }
    Ok(())
}

pub(super) fn safe_file_identity(file: &File) -> Result<FileIdentity, EvaluationLedgerError> {
    let metadata = file
        .metadata()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-stat-failed"))?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        length: metadata.len(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
    })
}

fn ensure_only_known_entries(
    path: &Path,
    root: &File,
    initializing: bool,
) -> Result<Option<PendingPublication>, EvaluationLedgerError> {
    test_directory_scan_pause(path);
    let names = read_directory_names(root)?;
    let allowed = [ANCHOR_NAME, LOCK_NAME, STATE_NAME];
    let mut pending = None;
    for name in names {
        if !initializing && allowed.iter().any(|allowed| name == OsStr::new(allowed)) {
            continue;
        }
        let Some(name) = name.to_str() else {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-unknown-or-pending-entry",
            ));
        };
        let Some(generation) = (!initializing)
            .then(|| pending_generation(name, STATE_NAME))
            .flatten()
        else {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-unknown-or-pending-entry",
            ));
        };
        if pending.is_some() {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-multiple-pending-publications",
            ));
        }
        let descriptor = openat(
            root,
            name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        let file = unsafe { File::from_raw_fd(descriptor) };
        let identity = safe_file_identity(&file)?;
        if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32
            || identity.links != 1
            || identity.mode & 0o777 != 0o600
            || identity.length > MAX_LEDGER_BYTES
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-pending-publication-unsafe",
            ));
        }
        pending = Some(PendingPublication {
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
    pending: Option<&PendingPublication>,
    snapshot: &AuthenticatedSnapshot,
) -> Result<(), EvaluationLedgerError> {
    if pending.is_some_and(|pending| pending.generation != snapshot.payload.core.generation) {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-pending-generation-invalid",
        ));
    }
    Ok(())
}

fn remove_pending_publication(
    root: &File,
    pending: &PendingPublication,
) -> Result<(), EvaluationLedgerError> {
    let descriptor = openat(
        root,
        &pending.name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        0,
    )?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    if safe_file_identity(&file)? != pending.identity {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-pending-publication-substituted",
        ));
    }
    let name = c_string(&pending.name)?;
    if unsafe { libc::unlinkat(root.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-pending-publication-remove-failed",
        ));
    }
    Ok(())
}

pub(super) fn read_json_file<T: for<'de> Deserialize<'de>>(
    root: &File,
    name: &str,
) -> Result<T, EvaluationLedgerError> {
    let descriptor = openat(
        root,
        name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        0,
    )?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file)?;
    let length = file
        .metadata()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-stat-failed"))?
        .len();
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32
        || identity.links != 1
        || identity.mode & 0o022 != 0
        || length == 0
        || length > MAX_LEDGER_BYTES
    {
        return Err(EvaluationLedgerError::new("evaluation-ledger-file-unsafe"));
    }
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(MAX_LEDGER_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-read-failed"))?;
    if bytes.len() as u64 != length || bytes.len() as u64 > MAX_LEDGER_BYTES {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-truncated-or-oversized",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-malformed"))
}

fn read_file_bytes(file: &File, maximum: u64) -> Result<Vec<u8>, EvaluationLedgerError> {
    use std::os::unix::fs::FileExt;

    let identity = safe_file_identity(file)?;
    if identity.length == 0 || identity.length > maximum {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-length-invalid",
        ));
    }
    let mut bytes = vec![0_u8; identity.length as usize];
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let read = file
            .read_at(&mut bytes[offset..], offset as u64)
            .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-read-failed"))?;
        if read == 0 {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-read-truncated",
            ));
        }
        offset += read;
    }
    Ok(bytes)
}

fn append_anchor_record(
    anchor: &File,
    record: &AuthenticatedAnchorRecord,
) -> Result<(FileIdentity, u64), EvaluationLedgerError> {
    use std::os::unix::fs::FileExt;

    let bytes = serde_json::to_vec(record)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    if bytes.is_empty() || bytes.len() > MAX_ANCHOR_RECORD_BYTES {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-record-length-invalid",
        ));
    }
    let before = safe_file_identity(anchor)?;
    let frame_length = 8_u64
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| EvaluationLedgerError::new("evaluation-anchor-frame-overflow"))?;
    let end = before
        .length
        .checked_add(frame_length)
        .filter(|end| *end <= MAX_ANCHOR_JOURNAL_BYTES)
        .ok_or_else(|| EvaluationLedgerError::new("evaluation-anchor-journal-full"))?;
    let header = (bytes.len() as u64).to_be_bytes();
    let mut written = 0_usize;
    while written < header.len() {
        let count = anchor
            .write_at(&header[written..], before.length + written as u64)
            .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-write-failed"))?;
        if count == 0 {
            return Err(EvaluationLedgerError::new("evaluation-anchor-write-failed"));
        }
        written += count;
    }
    written = 0;
    while written < bytes.len() {
        let count = anchor
            .write_at(
                &bytes[written..],
                before.length + header.len() as u64 + written as u64,
            )
            .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-write-failed"))?;
        if count == 0 {
            return Err(EvaluationLedgerError::new("evaluation-anchor-write-failed"));
        }
        written += count;
    }
    anchor
        .sync_all()
        .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-fsync-failed"))?;
    let after = safe_file_identity(anchor)?;
    if after.authority() != before.authority() || after.length != end {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-changed-during-append",
        ));
    }
    Ok((after, end))
}

pub(super) fn publish_file(
    root: &File,
    destination: &str,
    value: &impl Serialize,
    generation: u64,
) -> Result<(), EvaluationLedgerError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    let temporary = format!(
        ".{destination}.pending.{}.{}",
        std::process::id(),
        generation
    );
    let descriptor = openat(
        root,
        &temporary,
        libc::O_WRONLY
            | libc::O_CREAT
            | libc::O_EXCL
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK,
        0o600,
    )?;
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    file.write_all(&bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-write-failed"))?;
    file.sync_all()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-fsync-failed"))?;
    renameat(root, &temporary, destination)?;
    Ok(())
}

pub(super) fn entry_exists(root: &File, name: &str) -> Result<bool, EvaluationLedgerError> {
    let name = c_string(name)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            root.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(true);
    }
    if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
        return Ok(false);
    }
    Err(EvaluationLedgerError::new(
        "evaluation-ledger-entry-probe-failed",
    ))
}

pub(super) fn openat(
    root: &File,
    name: &str,
    flags: i32,
    mode: u32,
) -> Result<i32, EvaluationLedgerError> {
    let name = c_string(name)?;
    let descriptor = unsafe { libc::openat(root.as_raw_fd(), name.as_ptr(), flags, mode) };
    if descriptor < 0 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-openat-failed",
        ));
    }
    Ok(descriptor)
}

fn renameat(root: &File, source: &str, destination: &str) -> Result<(), EvaluationLedgerError> {
    let source = c_string(source)?;
    let destination = c_string(destination)?;
    if unsafe {
        libc::renameat(
            root.as_raw_fd(),
            source.as_ptr(),
            root.as_raw_fd(),
            destination.as_ptr(),
        )
    } != 0
    {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-rename-failed",
        ));
    }
    Ok(())
}

pub(super) fn sync_directory(root: &File) -> Result<(), EvaluationLedgerError> {
    root.sync_all()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-directory-fsync-failed"))
}

fn c_string(value: &str) -> Result<CString, EvaluationLedgerError> {
    CString::new(value)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-component-invalid"))
}

pub(super) fn hmac(bytes: &[u8], key: &[u8; 32]) -> Result<String, EvaluationLedgerError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-key-invalid"))?;
    mac.update(bytes);
    Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) struct FileLock<'a> {
    file: &'a File,
}

impl<'a> FileLock<'a> {
    pub(super) fn exclusive(file: &'a File) -> Result<Self, EvaluationLedgerError> {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(EvaluationLedgerError::new("evaluation-ledger-lock-failed"));
        }
        Ok(Self { file })
    }
}

impl Drop for FileLock<'_> {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
