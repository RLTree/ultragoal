//! Root-mediated one-use repository-fit apply authority.
//!
//! This module is intentionally internal. Permit issuance and public apply
//! dispatch remain root-owned and are not activated by this increment.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(unix)]
use std::ffi::{CStr, CString, OsStr};
use std::fmt::{Debug, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::sync::Mutex;

#[cfg(unix)]
use std::mem::MaybeUninit;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};

use crate::context::{BuildRequest, LiveContext};
#[cfg(unix)]
use crate::repository_fit::local::root_binding_from_canonical_path;
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, FitEffects, FitError, FitErrorId, FitReader, FitVerification,
    apply, digest, rollback, valid_digest, verify,
};

#[cfg(unix)]
use crate::repository_fit::LocalEffects;

use super::protocol::{ApplyRequestSeal, OpaqueFitApplyRequest, revalidate_apply_request};
use super::{AdapterErrorId, FitAdapterError, adapter_error};

const PERMIT_DOMAIN: &str = "repository-fit-root-apply-permit-v1";
const AUTHORITY_DOMAIN: &str = "repository-fit-root-apply-authority-v1";
const ROLLBACK_POLICY: &str = "complete-exact-prestate-or-ambiguous-v1";
pub(super) const MAX_PERMIT_LIFETIME: u64 = 300;
pub(super) const MIN_NONCE_BYTES: usize = 32;
const MAX_FENCE_ENTRIES: usize = 100_000;
const MAX_FENCE_DEPTH: usize = 64;
const MAX_FENCE_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FENCE_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const CREATED_MANAGED_ANCESTOR_MODE: u32 = 0o755;
const MAX_TARGET_ENTRY_SCAN: usize = 4 * 1024;
const MAX_TARGET_NAME_BYTES: usize = 1024 * 1024;
const MAX_MANAGED_ANCESTOR_CONTRACT_ROWS: usize = 512;

#[cfg(test)]
thread_local! {
    static BEFORE_POSTFLIGHT_OBSERVATION: RefCell<Option<Box<dyn FnOnce()>>> =
        RefCell::new(None);
    static BEFORE_FINAL_GREEN_OBSERVATION: RefCell<Option<Box<dyn FnOnce()>>> =
        RefCell::new(None);
    static TARGET_CAPTURE_HOOK: RefCell<Option<TargetCaptureHook>> = RefCell::new(None);
    static PROTECTED_CAPTURE_HOOK: RefCell<Option<ProtectedCaptureHook>> = RefCell::new(None);
    static RECONCILIATION_TARGET_HOOK: RefCell<Option<ReconciliationTargetHook>> =
        RefCell::new(None);
}

#[cfg(test)]
struct TargetCaptureHook {
    phase: TargetCapturePhase,
    path: String,
    action: Box<dyn FnOnce()>,
}

#[cfg(test)]
struct ProtectedCaptureHook {
    boundary: ProtectedCaptureBoundary,
    phase: ProtectedCapturePhase,
    path: Vec<u8>,
    action: Box<dyn FnOnce()>,
}

#[cfg(test)]
struct ReconciliationTargetHook {
    phase: ReconciliationTargetPhase,
    action: Box<dyn FnOnce()>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TargetCapturePhase {
    AfterNamedBeforeOpen,
    AfterParentHeldBeforeDescend,
    AfterLeafHeldBeforeRead,
    AfterLeafRevalidated,
    AfterMissingBeforeRecheck,
    BeforeFinalChainRecheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProtectedCaptureBoundary {
    PermitIssuance,
    PermitIssuanceRecheck,
    ImmediatePreEffect,
    ImmediatePreEffectRecheck,
    Postflight,
    PostflightRecheck,
    FinalGreen,
    FinalGreenRecheck,
    Rollback,
    AmbiguityReconciliation,
    AmbiguityReconciliationRecheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProtectedCapturePhase {
    AfterEnumerationBeforeChildOpen,
    AfterDirectoryHeldBeforeDescend,
    AfterRegularRowRevalidated,
    BeforeFinalRecheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReconciliationTargetPhase {
    AfterAuthorizedRevalidation,
    AfterFirstTarget,
    AfterProtectedAfter,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PermitBinding {
    request_id: String,
    context_id: String,
    candidate_id: String,
    repository_root_id: String,
    worktree_root_id: String,
    root_binding: String,
    plan_record_sha256: String,
    plan_sha256: String,
    accepted_plan_sha256: String,
    desired_state_sha256: String,
    source_manifest_sha256: String,
    source_catalog_sha256: String,
    source_authority_sha256: String,
    target_prestate_sha256: String,
    allowed_mutation_set_sha256: String,
    rollback_policy_sha256: String,
    protected_prestate_sha256: String,
}

pub(super) struct AuthorityIdentity {
    authority_id: String,
}

impl Debug for AuthorityIdentity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthorityIdentity")
            .field("authority_id", &"[bound]")
            .finish()
    }
}

impl AuthorityIdentity {
    pub(super) fn id(&self) -> &str {
        &self.authority_id
    }
}

/// Descriptor-bound pre-effect material prepared before the durable store is
/// initialized. Keeping the descriptor chains alive closes target replacement
/// between validation, reservation, and production lease activation.
pub(super) struct PreparedProductionPermit {
    binding: PermitBinding,
    target_prestate: TargetSnapshot,
    target_chain: TargetDescriptorChain,
    protected_prestate: ProtectedSnapshot,
}

/// Bounded durable identifiers for one exact permit reservation. No raw nonce,
/// path, target bytes, or authority key material crosses this projection.
pub(super) struct ProductionReservationBinding {
    binding_sha256: String,
    semantic_effect_id: String,
    target_scope_id: String,
    permit_id: String,
    nonce_sha256: String,
}

impl ProductionReservationBinding {
    pub(super) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(super) fn semantic_effect_id(&self) -> &str {
        &self.semantic_effect_id
    }

    pub(super) fn target_scope_id(&self) -> &str {
        &self.target_scope_id
    }

    pub(super) fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub(super) fn nonce_sha256(&self) -> &str {
        &self.nonce_sha256
    }
}

/// Opaque root authority for one exact request instance. It intentionally
/// implements neither Clone, Copy, Serialize, nor Deserialize.
#[must_use = "a repository-fit apply permit must be consumed or explicitly discarded"]
pub(crate) struct RepositoryFitApplyPermit {
    permit_id: String,
    binding: PermitBinding,
    issued_tick: u64,
    expires_tick: u64,
    nonce_sha256: String,
    authority: Arc<AuthorityIdentity>,
    seal: Arc<ApplyRequestSeal>,
    target_prestate: TargetSnapshot,
    target_chain: TargetDescriptorChain,
    protected_prestate: ProtectedSnapshot,
}

impl Debug for RepositoryFitApplyPermit {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepositoryFitApplyPermit")
            .field("permit_id", &"[bound]")
            .field("binding", &"[bound]")
            .field("issued_tick", &self.issued_tick)
            .field("expires_tick", &self.expires_tick)
            .field("nonce", &"[redacted]")
            .field("authority", &"[redacted]")
            .finish()
    }
}

/// Effect capability moved into exactly one mediation attempt. No constructor
/// exists in production until root-owned LocalEffects activation is wired.
#[must_use = "an exclusive repository-fit mutation lease must be consumed once"]
pub(crate) struct RepositoryFitMutationLease<E: RepositoryFitPermitEffects> {
    binding_id: String,
    authority: Arc<AuthorityIdentity>,
    seal: Arc<ApplyRequestSeal>,
    effects: ScopedEffects<E>,
}

impl<E: RepositoryFitPermitEffects> Debug for RepositoryFitMutationLease<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepositoryFitMutationLease")
            .field("binding", &"[bound]")
            .field("authority", &"[redacted]")
            .finish()
    }
}

pub(crate) trait RepositoryFitPermitEffects: FitEffects {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError>;
}

#[cfg(unix)]
impl RepositoryFitPermitEffects for LocalEffects {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        LocalEffects::read_unix_mode(self, path)
    }
}

struct ScopedEffects<E> {
    inner: E,
    allowed: Vec<AllowedMutation>,
    root: PathBuf,
    target_paths: Vec<CanonicalPath>,
    target_prestate: TargetSnapshot,
    authorized_target: TargetCapture,
    scope_violation: bool,
    chain_violation: bool,
}

#[derive(Clone)]
struct AllowedMutation {
    path: CanonicalPath,
    forward_expected: ExpectedContent,
    replacement: Vec<u8>,
    prior: Option<Vec<u8>>,
    forward_mode: u32,
    prior_mode: Option<u32>,
}

impl<E> ScopedEffects<E> {
    fn new(
        inner: E,
        request: &OpaqueFitApplyRequest,
        root: &Path,
        target_prestate: TargetSnapshot,
        authorized_target: TargetCapture,
    ) -> Self {
        let allowed = request
            .plan
            .mutations
            .iter()
            .map(|mutation| AllowedMutation {
                path: mutation.path.clone(),
                forward_expected: mutation.expected.clone(),
                replacement: mutation.replacement.clone(),
                prior: mutation.prior.clone(),
                forward_mode: request.unix_modes[mutation.path.as_str()],
                prior_mode: request.observed_modes[mutation.path.as_str()],
            })
            .collect();
        Self {
            inner,
            allowed,
            root: root.to_path_buf(),
            target_paths: request
                .desired
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect(),
            target_prestate,
            authorized_target,
            scope_violation: false,
            chain_violation: false,
        }
    }

    fn scope_violation(&self) -> bool {
        self.scope_violation || self.chain_violation
    }

    fn capture_authorized(&mut self) -> Result<TargetCapture, FitError> {
        if self.authorized_target.chain.revalidate().is_err() {
            self.chain_violation = true;
            return Err(chain_fit_error());
        }
        let current = capture_target_descriptor_chain_for_paths(&self.root, &self.target_paths)
            .map_err(|_| chain_fit_error())?;
        if current.snapshot != self.authorized_target.snapshot {
            self.chain_violation = true;
            return Err(chain_fit_error());
        }
        Ok(current)
    }

    fn revalidate_authorized_target(&mut self) -> Result<TargetSnapshot, FitAdapterError> {
        self.capture_authorized()
            .map(|capture| capture.snapshot)
            .map_err(|_| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))
    }
}

fn chain_fit_error() -> FitError {
    crate::repository_fit::error(FitErrorId::StaleBinding)
}

impl<E: RepositoryFitPermitEffects> FitReader for ScopedEffects<E> {
    fn root_binding(&mut self) -> Result<String, FitError> {
        self.inner.root_binding()
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.inner.read_file(path, maximum_bytes)
    }
}

impl<E: RepositoryFitPermitEffects> FitEffects for ScopedEffects<E> {
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        let allowed = self.allowed.iter().any(|row| {
            if &row.path != path {
                return false;
            }
            let forward = &row.forward_expected == expected
                && replacement == Some(row.replacement.as_slice());
            let rollback = *expected == ExpectedContent::ExactDigest(digest(&row.replacement))
                && replacement == row.prior.as_deref();
            forward || rollback
        });
        if !allowed {
            self.scope_violation = true;
            return Err(crate::repository_fit::error(FitErrorId::Unauthorized));
        }
        let before = self.capture_authorized()?;
        let row = self
            .allowed
            .iter()
            .find(|row| &row.path == path)
            .expect("the allowed predicate found this exact path")
            .clone();
        let forward =
            row.forward_expected == *expected && replacement == Some(row.replacement.as_slice());
        let rollback = *expected == ExpectedContent::ExactDigest(digest(&row.replacement))
            && replacement == row.prior.as_deref();
        let before_mode =
            target_object(&before.snapshot, path.as_str()).map(|object| object.mode & 0o7777);
        let expected_mode = match (forward, rollback) {
            (true, true) if before_mode == Some(row.forward_mode) => row.prior_mode,
            (true, _) => Some(row.forward_mode),
            (_, true) => row.prior_mode,
            _ => None,
        };
        let result = self.inner.compare_exchange(path, expected, replacement);
        let after = match capture_target_descriptor_chain_for_paths(&self.root, &self.target_paths)
        {
            Ok(after) => after,
            Err(_) => {
                self.chain_violation = true;
                return Err(chain_fit_error());
            }
        };
        match result {
            Ok(false) if after.snapshot == before.snapshot => {
                self.authorized_target = after;
                Ok(false)
            }
            Ok(true)
                if valid_target_effect_transition(
                    &before.snapshot,
                    &after.snapshot,
                    &self.target_prestate,
                    path,
                    replacement,
                    expected_mode,
                ) =>
            {
                self.authorized_target = after;
                Ok(true)
            }
            // An untrusted effect adapter may claim success without changing
            // the namespace. Preserve that report so the kernel's desired
            // state proof rejects it and causal reconciliation can still
            // prove the exact prior state rather than manufacturing ambiguity.
            Ok(true) if after.snapshot == before.snapshot => {
                self.authorized_target = after;
                Ok(true)
            }
            Err(failure) if after.snapshot == before.snapshot => {
                self.authorized_target = after;
                Err(failure)
            }
            Ok(_) | Err(_) => {
                self.chain_violation = true;
                Err(chain_fit_error())
            }
        }
    }
}

impl<E: RepositoryFitPermitEffects> RepositoryFitPermitEffects for ScopedEffects<E> {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        self.inner.read_unix_mode(path)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TargetSnapshot {
    sha256: String,
    rows: Vec<TargetRow>,
}

/// One internally consistent target observation. Every present component was
/// opened relative to its held parent descriptor and every absent boundary is
/// retained for an exact-name recheck. Keeping this value alive keeps the
/// complete root-to-leaf descriptor chains alive as well.
struct TargetCapture {
    snapshot: TargetSnapshot,
    chain: TargetDescriptorChain,
}

struct TargetDescriptorChain {
    root_path: PathBuf,
    root: File,
    root_object: ObjectRow,
    paths: Vec<HeldTargetPath>,
}

struct HeldTargetPath {
    attachments: Vec<HeldTargetAttachment>,
    missing: Option<HeldMissingAttachment>,
}

struct HeldTargetAttachment {
    path: String,
    parent: File,
    name: String,
    object: ObjectRow,
    opened: Option<File>,
}

struct HeldMissingAttachment {
    path: String,
    parent: File,
    name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExactEntry {
    Absent,
    Exact,
    Alias,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TargetRow {
    path: String,
    state: TargetState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum TargetState {
    Missing,
    Present(ObjectRow),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ObjectRow {
    kind: &'static str,
    device: u64,
    inode: u64,
    links: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    byte_length: u64,
    payload_sha256: Option<String>,
    change_version: ProtectedChangeVersion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ManagedAncestorContract {
    rows: Vec<ManagedAncestorContractRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedAncestorContractRow {
    path: String,
    expectation: ManagedAncestorExpectation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum ManagedAncestorExpectation {
    Existing {
        device: u64,
        inode: u64,
        uid: u32,
        gid: u32,
        mode: u32,
    },
    Missing,
}

pub(super) struct RecoveryTargetContractObservation {
    pub(super) root_binding: String,
    pub(super) ancestor_preimage: bool,
    pub(super) ancestor_postimage: bool,
    pub(super) leaves: Vec<RecoveryLeafObservation>,
}

pub(super) struct RecoveryLeafObservation {
    pub(super) path: String,
    pub(super) payload_sha256: Option<String>,
    pub(super) mode: Option<u32>,
    pub(super) valid_managed_leaf: bool,
}

impl ManagedAncestorContract {
    pub(super) fn valid_for_leaf_paths(&self, leaf_paths: &[String]) -> bool {
        if self.rows.is_empty() || self.rows.len() > MAX_MANAGED_ANCESTOR_CONTRACT_ROWS {
            return false;
        }
        let targets = match leaf_paths
            .iter()
            .map(|path| CanonicalPath::parse(path))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(targets) => targets,
            Err(_) => return false,
        };
        let expected_paths = managed_ancestor_paths_for_targets(&targets);
        if self
            .rows
            .iter()
            .map(|row| row.path.as_str())
            .ne(expected_paths.iter().map(String::as_str))
        {
            return false;
        }
        self.rows.iter().enumerate().all(|(index, row)| {
            if index == 0 && row.path != "" {
                return false;
            }
            match row.expectation {
                ManagedAncestorExpectation::Existing {
                    device,
                    inode,
                    mode,
                    ..
                } => device != 0 && inode != 0 && mode & 0o170000 == 0o040000,
                ManagedAncestorExpectation::Missing => !row.path.is_empty(),
            }
        })
    }
}

#[cfg(test)]
pub(super) fn managed_ancestor_contract_for_ledger_test() -> ManagedAncestorContract {
    ManagedAncestorContract {
        rows: vec![ManagedAncestorContractRow {
            path: String::new(),
            expectation: ManagedAncestorExpectation::Existing {
                device: 1,
                inode: 1,
                uid: 0,
                gid: 0,
                mode: 0o040755,
            },
        }],
    }
}

impl ObjectRow {
    fn managed_ancestor_equivalent(&self, other: &Self) -> bool {
        self.kind == "directory"
            && other.kind == "directory"
            && self.device == other.device
            && self.inode == other.inode
            && self.uid == other.uid
            && self.gid == other.gid
            && self.mode == other.mode
    }

    fn valid_created_managed_ancestor(&self, root: &Self) -> bool {
        self.kind == "directory"
            && self.device == root.device
            && self.uid == root.uid
            && self.gid == root.gid
            && self.mode & 0o7777 == CREATED_MANAGED_ANCESTOR_MODE
    }

    fn rollback_equivalent(&self, other: &Self, leaf: bool) -> bool {
        self.kind == other.kind
            && self.device == other.device
            && (self.inode == other.inode || (leaf && self.kind == "regular"))
            && self.links == other.links
            && self.uid == other.uid
            && self.gid == other.gid
            && self.mode == other.mode
            && self.byte_length == other.byte_length
            && self.payload_sha256 == other.payload_sha256
    }
}

fn target_object<'a>(snapshot: &'a TargetSnapshot, path: &str) -> Option<&'a ObjectRow> {
    snapshot
        .rows
        .iter()
        .find(|row| row.path == path)
        .and_then(|row| match &row.state {
            TargetState::Present(object) => Some(object),
            TargetState::Missing => None,
        })
}

fn valid_target_effect_transition(
    before: &TargetSnapshot,
    after: &TargetSnapshot,
    permit_prestate: &TargetSnapshot,
    mutation_path: &CanonicalPath,
    replacement: Option<&[u8]>,
    expected_mode: Option<u32>,
) -> bool {
    let before_rows = before
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let after_rows = after
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let permit_rows = permit_prestate
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    if before_rows.keys().ne(after_rows.keys()) || before_rows.keys().ne(permit_rows.keys()) {
        return false;
    }
    let root = match present_target_row(&after_rows, "") {
        Some(root) => root,
        None => return false,
    };
    let target = mutation_path.as_str();
    for (path, before_state) in before_rows {
        let after_state = after_rows[path];
        if path == target {
            let valid_leaf = match (replacement, expected_mode, after_state) {
                (None, _, TargetState::Missing) => true,
                (Some(bytes), Some(mode), TargetState::Present(object)) => {
                    object.kind == "regular"
                        && object.device == root.device
                        && object.links == 1
                        && object.uid == root.uid
                        && object.gid == root.gid
                        && object.mode & 0o7777 == mode
                        && object.byte_length == bytes.len() as u64
                        && object.payload_sha256.as_deref() == Some(digest(bytes).as_str())
                }
                _ => false,
            };
            if !valid_leaf {
                return false;
            }
            continue;
        }
        let strict_ancestor = path.is_empty()
            || (target.len() > path.len()
                && target.starts_with(path)
                && target.as_bytes().get(path.len()) == Some(&b'/'));
        if !strict_ancestor {
            if before_state != after_state {
                return false;
            }
            continue;
        }
        let valid_ancestor = match (before_state, after_state, permit_rows[path]) {
            (TargetState::Present(before), TargetState::Present(after), _) => {
                before.managed_ancestor_equivalent(after)
            }
            (TargetState::Missing, TargetState::Present(after), TargetState::Missing) => {
                after.valid_created_managed_ancestor(root)
            }
            (TargetState::Present(_), TargetState::Missing, TargetState::Missing) => true,
            (TargetState::Missing, TargetState::Missing, _) => true,
            _ => false,
        };
        if !valid_ancestor {
            return false;
        }
    }
    true
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ProtectedSnapshot {
    sha256: String,
    rows: Vec<ProtectedRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ProtectedRow {
    path: Vec<u8>,
    object: ObjectRow,
    change_version: ProtectedChangeVersion,
}

/// Darwin's status-change timestamp is kernel-maintained and cannot be reset
/// by an unprivileged repository writer. Binding both components to every
/// target and protected object closes same-inode content and metadata ABA that
/// payload or ordinary identity alone cannot distinguish after the original
/// bytes or mode have been restored.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct ProtectedChangeVersion {
    ctime_seconds: i64,
    ctime_nanoseconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VersionedProtectedObject {
    object: ObjectRow,
    change_version: ProtectedChangeVersion,
}

struct FenceBudget {
    entries: usize,
    name_bytes: usize,
    bytes: u64,
}

/// Successful mediation output can only be constructed after the accepted
/// kernel effect and both target and context reconciliation have succeeded.
#[derive(Debug, Serialize)]
pub(crate) struct RepositoryFitApplyOutcome {
    schema_version: &'static str,
    outcome_id: String,
    request_id: String,
    plan_sha256: String,
    desired_state_sha256: String,
    verification_sha256: String,
    target_poststate_sha256: String,
    mutation_count: usize,
    status: &'static str,
    effect: &'static str,
    claim_effect: &'static str,
    support_limit: &'static str,
}

impl RepositoryFitApplyOutcome {
    pub(crate) fn mutation_count(&self) -> usize {
        self.mutation_count
    }

    pub(crate) fn status(&self) -> &str {
        self.status
    }

    pub(crate) fn outcome_id(&self) -> &str {
        &self.outcome_id
    }
}

pub(crate) enum RepositoryFitApplyFailure<E: RepositoryFitPermitEffects> {
    PreEffect(PreEffectFailure<E>),
    Terminal(TerminalApplyFailure),
}

impl<E: RepositoryFitPermitEffects> RepositoryFitApplyFailure<E> {
    pub(crate) fn error(&self) -> FitAdapterError {
        match self {
            Self::PreEffect(failure) => failure.error,
            Self::Terminal(failure) => failure.error,
        }
    }

    pub(crate) fn effect_started(&self) -> bool {
        matches!(self, Self::Terminal(failure) if failure.effect_started)
    }

    pub(crate) fn rollback_complete(&self) -> bool {
        matches!(self, Self::Terminal(failure) if failure.rollback_complete)
    }

    pub(crate) fn into_pre_effect(self) -> Option<PreEffectFailure<E>> {
        match self {
            Self::PreEffect(failure) => Some(failure),
            Self::Terminal(_) => None,
        }
    }
}

pub(crate) struct PreEffectFailure<E: RepositoryFitPermitEffects> {
    error: FitAdapterError,
    request: OpaqueFitApplyRequest,
    permit: Option<RepositoryFitApplyPermit>,
    lease: Option<RepositoryFitMutationLease<E>>,
}

impl<E: RepositoryFitPermitEffects> PreEffectFailure<E> {
    pub(crate) fn into_parts(
        self,
    ) -> (
        OpaqueFitApplyRequest,
        Option<RepositoryFitApplyPermit>,
        Option<RepositoryFitMutationLease<E>>,
    ) {
        (self.request, self.permit, self.lease)
    }
}

pub(crate) struct TerminalApplyFailure {
    error: FitAdapterError,
    effect_started: bool,
    rollback_complete: bool,
}

/// Consumes the exact opaque request and exclusive lease. Every refusal before
/// the seal transition returns both intact. Once the transition succeeds, no
/// path returns reusable mutation authority.
pub(crate) fn apply_with_root_permit<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: OpaqueFitApplyRequest,
    mut permit: Option<RepositoryFitApplyPermit>,
    mut lease: Option<RepositoryFitMutationLease<E>>,
    now_tick: u64,
) -> Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>> {
    let preflight = preflight(
        context,
        &request,
        permit.as_mut(),
        lease.as_mut().map(|lease| lease),
        now_tick,
    );
    if let Err(error) = preflight {
        return Err(RepositoryFitApplyFailure::PreEffect(PreEffectFailure {
            error,
            request,
            permit,
            lease,
        }));
    }
    let permit = permit.expect("preflight requires a permit");
    let mut lease = lease.expect("preflight requires a lease");
    if let Err(error) = request.seal.begin() {
        return Err(RepositoryFitApplyFailure::Terminal(TerminalApplyFailure {
            error,
            effect_started: true,
            rollback_complete: false,
        }));
    }

    let mutation_count = request.plan.mutations.len();
    match apply(&request.plan, &request.authorization, &mut lease.effects) {
        Ok(transaction) => {
            let postflight = postflight(context, &request, &permit, &mut lease.effects);
            match postflight {
                Ok((verification, target_poststate)) => {
                    test_before_final_green_observation();
                    let final_target = match finalize_green(
                        context,
                        &request,
                        &permit,
                        &target_poststate,
                        &mut lease.effects,
                    ) {
                        Ok(final_target) => final_target,
                        Err(_) => {
                            let scope_violation = lease.effects.scope_violation();
                            return terminal_after_started_failure(
                                context,
                                &request,
                                &permit,
                                &mut lease.effects,
                                Some(transaction),
                                scope_violation,
                            );
                        }
                    };
                    if request.seal.settle().is_err() {
                        return terminal_ambiguous();
                    }
                    Ok(success_outcome(
                        &request,
                        &permit,
                        &verification,
                        &final_target,
                        mutation_count,
                    ))
                }
                Err(_) => {
                    let scope_violation = lease.effects.scope_violation();
                    terminal_after_started_failure(
                        context,
                        &request,
                        &permit,
                        &mut lease.effects,
                        Some(transaction),
                        scope_violation,
                    )
                }
            }
        }
        Err(_) => {
            let scope_violation = lease.effects.scope_violation();
            terminal_after_started_failure(
                context,
                &request,
                &permit,
                &mut lease.effects,
                None,
                scope_violation,
            )
        }
    }
}

#[cfg(test)]
fn test_before_final_green_observation() {
    BEFORE_FINAL_GREEN_OBSERVATION.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(test)]
fn test_before_postflight_observation() {
    BEFORE_POSTFLIGHT_OBSERVATION.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
const fn test_before_postflight_observation() {}

#[cfg(not(test))]
const fn test_before_final_green_observation() {}

#[cfg(test)]
fn test_target_capture_point(phase: TargetCapturePhase, path: &str) {
    let action = TARGET_CAPTURE_HOOK.with(|slot| {
        let mut hook = slot.borrow_mut();
        if hook
            .as_ref()
            .is_some_and(|hook| hook.phase == phase && hook.path == path)
        {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
const fn test_target_capture_point(_phase: TargetCapturePhase, _path: &str) {}

#[cfg(test)]
fn test_protected_capture_point(
    boundary: ProtectedCaptureBoundary,
    phase: ProtectedCapturePhase,
    path: &[u8],
) {
    let action = PROTECTED_CAPTURE_HOOK.with(|slot| {
        let mut hook = slot.borrow_mut();
        if hook.as_ref().is_some_and(|hook| {
            hook.boundary == boundary && hook.phase == phase && hook.path == path
        }) {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(test)]
fn test_reconciliation_target_point(phase: ReconciliationTargetPhase) {
    let action = RECONCILIATION_TARGET_HOOK.with(|slot| {
        let mut hook = slot.borrow_mut();
        if hook.as_ref().is_some_and(|hook| hook.phase == phase) {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
const fn test_reconciliation_target_point(_phase: ReconciliationTargetPhase) {}

#[cfg(not(test))]
const fn test_protected_capture_point(
    _boundary: ProtectedCaptureBoundary,
    _phase: ProtectedCapturePhase,
    _path: &[u8],
) {
}

fn preflight<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: Option<&mut RepositoryFitApplyPermit>,
    lease: Option<&mut RepositoryFitMutationLease<E>>,
    now_tick: u64,
) -> Result<(), FitAdapterError> {
    let permit = permit.ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitMissing))?;
    let lease = lease.ok_or_else(|| adapter_error(AdapterErrorId::ApplyLeaseInvalid))?;
    if now_tick < permit.issued_tick || now_tick > permit.expires_tick {
        return Err(adapter_error(AdapterErrorId::ApplyPermitExpired));
    }
    let expected_binding =
        permit_binding(request, &permit.target_prestate, &permit.protected_prestate)?;
    let expected_permit_id = permit_id(
        &permit.authority.authority_id,
        &expected_binding,
        permit.issued_tick,
        permit.expires_tick,
        &permit.nonce_sha256,
    )?;
    if permit.binding != expected_binding
        || permit.permit_id != expected_permit_id
        || !Arc::ptr_eq(&request.seal, &permit.seal)
        || !request.seal_matches(&request.seal_id())
    {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    if !Arc::ptr_eq(&request.seal, &lease.seal)
        || !Arc::ptr_eq(&permit.authority, &lease.authority)
        || lease.binding_id != permit.binding.plan_record_sha256
    {
        return Err(adapter_error(AdapterErrorId::ApplyLeaseInvalid));
    }
    let protected_before = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::ImmediatePreEffect,
    )?;
    if protected_before != permit.protected_prestate {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    permit
        .target_chain
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::StalePlan))?;
    revalidate_apply_request(context, request)?;
    require_root_binding(&mut lease.effects, &request.root_binding)?;
    let target = capture_target_descriptor_chain(context.worktree_root(), request)?.snapshot;
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    require_root_binding(&mut lease.effects, &request.root_binding)?;
    let protected_after = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::ImmediatePreEffectRecheck,
    )?;
    let target_after = capture_target(context.worktree_root(), request)?;
    if target != permit.target_prestate
        || target_after != permit.target_prestate
        || target != target_after
        || protected_after != permit.protected_prestate
        || protected_before != protected_after
    {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    Ok(())
}

fn postflight<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    effects: &mut ScopedEffects<E>,
) -> Result<(FitVerification, TargetSnapshot), FitAdapterError> {
    test_before_postflight_observation();
    if effects.scope_violation() {
        return Err(adapter_error(AdapterErrorId::ApplyMutationScopeViolation));
    }
    let protected_before = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::Postflight,
    )?;
    if protected_before != permit.protected_prestate {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    effects.revalidate_authorized_target()?;
    require_root_binding(effects, &request.root_binding)?;
    let verification = verify(&request.desired, effects).map_err(super::kernel_error)?;
    let target_poststate =
        require_desired_target(context.worktree_root(), request, permit, effects)?;
    revalidate_post_context(context)?;
    require_root_binding(effects, &request.root_binding)?;
    let protected_after = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::PostflightRecheck,
    )?;
    if protected_after != permit.protected_prestate || protected_before != protected_after {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    let target_recheck = require_desired_target(context.worktree_root(), request, permit, effects)?;
    if target_recheck != target_poststate {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    Ok((verification, target_recheck))
}

/// Performs the final finite observation before success settlement. Equal
/// versioned protected observations bracket the first complete target capture,
/// and an equal complete target recheck follows the protected-after capture.
/// The protected and target stability intervals therefore overlap at one
/// common final-green linearization point.
fn finalize_green<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    expected_target: &TargetSnapshot,
    effects: &mut ScopedEffects<E>,
) -> Result<TargetSnapshot, FitAdapterError> {
    if effects.scope_violation() {
        return Err(adapter_error(AdapterErrorId::ApplyMutationScopeViolation));
    }
    let protected_before = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::FinalGreen,
    )?;
    if protected_before != permit.protected_prestate {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    effects.revalidate_authorized_target()?;
    revalidate_post_context(context)?;
    require_root_binding(effects, &request.root_binding)?;
    let final_target = require_desired_target(context.worktree_root(), request, permit, effects)?;
    let protected_after = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::FinalGreenRecheck,
    )?;
    if protected_after != permit.protected_prestate || protected_before != protected_after {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    let final_target_recheck =
        require_desired_target(context.worktree_root(), request, permit, effects)?;
    if &final_target != expected_target
        || final_target_recheck != final_target
        || &final_target_recheck != expected_target
    {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    Ok(final_target_recheck)
}

fn terminal_after_started_failure<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    effects: &mut ScopedEffects<E>,
    transaction: Option<crate::repository_fit::AppliedFit>,
    scope_violation: bool,
) -> Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>> {
    let chain_ready = effects.revalidate_authorized_target().is_ok();
    let protected_ready = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::Rollback,
    )
    .is_ok_and(|protected| protected == permit.protected_prestate);
    let rollback_result = if chain_ready && protected_ready {
        transaction.map_or(Ok(()), |transaction| rollback(transaction, effects))
    } else {
        Err(crate::repository_fit::error(FitErrorId::RollbackFailed))
    };
    let reconciled = rollback_result.is_ok() && reconcile_prior(context, request, permit, effects);
    if reconciled && request.seal.rolled_back().is_ok() {
        let error = if scope_violation {
            adapter_error(AdapterErrorId::ApplyMutationScopeViolation)
        } else {
            adapter_error(AdapterErrorId::ApplyRolledBack)
        };
        Err(RepositoryFitApplyFailure::Terminal(TerminalApplyFailure {
            error,
            effect_started: true,
            rollback_complete: true,
        }))
    } else {
        let _ = request.seal.ambiguous();
        terminal_ambiguous()
    }
}

fn terminal_ambiguous<E: RepositoryFitPermitEffects>()
-> Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>> {
    Err(RepositoryFitApplyFailure::Terminal(TerminalApplyFailure {
        error: adapter_error(AdapterErrorId::ApplyOutcomeAmbiguous),
        effect_started: true,
        rollback_complete: false,
    }))
}

fn reconcile_prior<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    effects: &mut ScopedEffects<E>,
) -> bool {
    let protected_before = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::AmbiguityReconciliation,
    );
    // Retain the exact snapshot recorded after the last mediated effect. Only
    // observations equal to this ctime-bound state may then use semantic
    // rollback equivalence against the permit prestate.
    let authorized_target = match effects.revalidate_authorized_target() {
        Ok(authorized_target) => authorized_target,
        Err(_) => return false,
    };
    test_reconciliation_target_point(ReconciliationTargetPhase::AfterAuthorizedRevalidation);
    let target = capture_target(context.worktree_root(), request);
    test_reconciliation_target_point(ReconciliationTargetPhase::AfterFirstTarget);
    let root = require_root_binding(effects, &request.root_binding);
    let context_valid = context.revalidate();
    let protected_after = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::AmbiguityReconciliationRecheck,
    );
    test_reconciliation_target_point(ReconciliationTargetPhase::AfterProtectedAfter);
    let target_after = capture_target(context.worktree_root(), request);
    matches!((protected_before, target, root, context_valid, protected_after, target_after),
        (Ok(protected_before), Ok(target), Ok(()), Ok(()), Ok(protected_after), Ok(target_after))
            if target == authorized_target
                && target_after == authorized_target
                && target_rollback_equivalent(&authorized_target, &permit.target_prestate, request)
                && protected_before == permit.protected_prestate
                && protected_after == permit.protected_prestate
                && protected_before == protected_after)
}

fn success_outcome(
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    verification: &FitVerification,
    target_poststate: &TargetSnapshot,
    mutation_count: usize,
) -> RepositoryFitApplyOutcome {
    let status = if mutation_count == 0 {
        "idempotent"
    } else {
        "applied"
    };
    let outcome_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-apply-outcome-v1",
            &request.request_id,
            &request.plan.plan_sha256,
            &request.desired.state_sha256,
            &verification.verification_sha256,
            &target_poststate.sha256,
            &permit.protected_prestate.sha256,
            mutation_count,
            status,
        ))
        .expect("fixed outcome payload is serializable"),
    );
    RepositoryFitApplyOutcome {
        schema_version: "RepositoryFitApplyOutcome-v1",
        outcome_id,
        request_id: request.request_id.clone(),
        plan_sha256: request.plan.plan_sha256.clone(),
        desired_state_sha256: request.desired.state_sha256.clone(),
        verification_sha256: verification.verification_sha256.clone(),
        target_poststate_sha256: target_poststate.sha256.clone(),
        mutation_count,
        status,
        effect: "workspace_write",
        claim_effect: "none",
        support_limit: "internal permit mediation only; root issuance and public apply dispatch absent",
    }
}

fn require_root_binding(
    effects: &mut impl FitReader,
    expected: &str,
) -> Result<(), FitAdapterError> {
    match effects.root_binding() {
        Ok(observed) if observed == expected => Ok(()),
        _ => Err(adapter_error(AdapterErrorId::ContextStale)),
    }
}

fn revalidate_post_context(context: &LiveContext) -> Result<(), FitAdapterError> {
    let mut request = BuildRequest::new(context.worktree_root())
        .expect_repository_root(PathBuf::from(&context.roots().repository_root))
        .expect_worktree_root(PathBuf::from(&context.roots().worktree_root));
    for (key, value) in &context.configuration().public_values {
        request = request.bind_non_secret_configuration(key.clone(), value.clone());
    }
    for source in &context.configuration().secret_sources {
        request = request.bind_secret_source(source.name.clone(), source.public_version.clone());
    }
    for input in context.selected_inputs() {
        request = request.select_input(input.relative_path.clone());
    }
    for tool in &context.capabilities().tools {
        request = request.probe_tool(tool.name.clone());
    }
    let current =
        LiveContext::build(request).map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let before = context.candidate();
    let after = current.candidate();
    if current.roots() != context.roots()
        || current.configuration() != context.configuration()
        || current.capabilities() != context.capabilities()
        || current.permissions() != context.permissions()
        || current.selected_inputs() != context.selected_inputs()
        || after.head_commit != before.head_commit
        || after.head_tree != before.head_tree
        || after.branch != before.branch
        || after.staged_diff_sha256 != before.staged_diff_sha256
    {
        return Err(adapter_error(AdapterErrorId::ContextStale));
    }
    Ok(())
}

fn permit_binding(
    request: &OpaqueFitApplyRequest,
    target: &TargetSnapshot,
    protected: &ProtectedSnapshot,
) -> Result<PermitBinding, FitAdapterError> {
    let mutation_rows = request
        .plan
        .mutations
        .iter()
        .map(|mutation| {
            Ok((
                mutation.path.as_str(),
                &mutation.expected,
                mutation.replacement_sha256(),
                mutation.prior.as_deref().map(digest),
                request
                    .unix_modes
                    .get(mutation.path.as_str())
                    .copied()
                    .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?,
                request
                    .observed_modes
                    .get(mutation.path.as_str())
                    .copied()
                    .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?,
            ))
        })
        .collect::<Result<Vec<_>, FitAdapterError>>()?;
    let allowed_mutation_set_sha256 = digest(
        &serde_json::to_vec(&mutation_rows)
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let rollback_policy_sha256 = digest(
        &serde_json::to_vec(&(
            ROLLBACK_POLICY,
            &mutation_rows,
            request.plan.rollback.mutation_count,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    Ok(PermitBinding {
        request_id: request.request_id.clone(),
        context_id: request.context_id.clone(),
        candidate_id: request.candidate_id.clone(),
        repository_root_id: request.target.repository_root_id.clone(),
        worktree_root_id: request.target.worktree_root_id.clone(),
        root_binding: request.root_binding.clone(),
        plan_record_sha256: digest(&request.plan_record_bytes),
        plan_sha256: request.plan.plan_sha256.clone(),
        accepted_plan_sha256: request.accepted_plan_sha256.clone(),
        desired_state_sha256: request.desired.state_sha256.clone(),
        source_manifest_sha256: request.authority.manifest_sha256.clone(),
        source_catalog_sha256: request.authority.catalog_sha256.clone(),
        source_authority_sha256: request.authority.authority_sha256.clone(),
        target_prestate_sha256: target.sha256.clone(),
        allowed_mutation_set_sha256,
        rollback_policy_sha256,
        protected_prestate_sha256: protected.sha256.clone(),
    })
}

fn permit_id(
    authority_id: &str,
    binding: &PermitBinding,
    issued_tick: u64,
    expires_tick: u64,
    nonce_sha256: &str,
) -> Result<String, FitAdapterError> {
    serde_json::to_vec(&(
        PERMIT_DOMAIN,
        authority_id,
        binding,
        issued_tick,
        expires_tick,
        nonce_sha256,
    ))
    .map(|bytes| digest(&bytes))
    .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))
}

fn capture_target(
    root: &Path,
    request: &OpaqueFitApplyRequest,
) -> Result<TargetSnapshot, FitAdapterError> {
    capture_target_descriptor_chain(root, request).map(|capture| capture.snapshot)
}

fn capture_target_descriptor_chain(
    root: &Path,
    request: &OpaqueFitApplyRequest,
) -> Result<TargetCapture, FitAdapterError> {
    let paths = request
        .desired
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    capture_target_descriptor_chain_for_paths(root, &paths)
}

fn capture_target_descriptor_chain_for_paths(
    root: &Path,
    target_paths: &[CanonicalPath],
) -> Result<TargetCapture, FitAdapterError> {
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (root, target_paths);
        return Err(adapter_error(AdapterErrorId::UnsupportedHost));
    }
    #[cfg(target_vendor = "apple")]
    {
        let first = collect_target_descriptor_chain_for_paths(root, target_paths)?;
        test_target_capture_point(TargetCapturePhase::BeforeFinalChainRecheck, "");
        let final_recheck = collect_target_descriptor_chain_for_paths(root, target_paths)?;
        if first.snapshot != final_recheck.snapshot {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        Ok(final_recheck)
    }
}

/// A complete descriptor-relative target collect. The public capture accepts
/// only two equal complete collects. Because each target object's Darwin ctime
/// is part of its row, the per-object stability intervals overlap at the
/// boundary between the collects and establish one common stable instant.
#[cfg(target_vendor = "apple")]
fn collect_target_descriptor_chain_for_paths(
    root: &Path,
    target_paths: &[CanonicalPath],
) -> Result<TargetCapture, FitAdapterError> {
    let root_path_metadata =
        fs::symlink_metadata(root).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    if root_path_metadata.file_type().is_symlink() || !root_path_metadata.is_dir() {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK);
    let root_file = options
        .open(root)
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let root_opened = root_file
        .metadata()
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let root_object = metadata_object(&root_opened, None)?;
    if root_object.kind != "directory" || metadata_object(&root_path_metadata, None)? != root_object
    {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }

    let mut rows = BTreeMap::from([(String::new(), TargetState::Present(root_object.clone()))]);
    let mut paths = Vec::with_capacity(target_paths.len());
    for target_path in target_paths {
        let components = target_path.components().collect::<Vec<_>>();
        let mut parent = root_file
            .try_clone()
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        let mut relative = String::new();
        let mut attachments = Vec::with_capacity(components.len());
        let mut missing = None;
        for (index, component) in components.iter().enumerate() {
            if !relative.is_empty() {
                relative.push('/');
            }
            relative.push_str(component);
            if missing.is_some() {
                insert_target_state(&mut rows, &relative, TargetState::Missing)?;
                continue;
            }
            match exact_entry_at(&parent, component)? {
                ExactEntry::Alias => {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                ExactEntry::Absent => {
                    if named_object_at(&parent, component, None)?.is_some() {
                        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                    }
                    test_target_capture_point(
                        TargetCapturePhase::AfterMissingBeforeRecheck,
                        &relative,
                    );
                    insert_target_state(&mut rows, &relative, TargetState::Missing)?;
                    missing = Some(HeldMissingAttachment {
                        path: relative.clone(),
                        parent: parent
                            .try_clone()
                            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
                        name: (*component).to_owned(),
                    });
                }
                ExactEntry::Exact => {
                    let leaf = index + 1 == components.len();
                    let attachment =
                        capture_target_attachment(&parent, component, &relative, !leaf)?;
                    if !leaf && attachment.object.kind != "directory" {
                        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                    }
                    insert_target_state(
                        &mut rows,
                        &relative,
                        TargetState::Present(attachment.object.clone()),
                    )?;
                    if leaf {
                        test_target_capture_point(
                            TargetCapturePhase::AfterLeafRevalidated,
                            &relative,
                        );
                    } else {
                        test_target_capture_point(
                            TargetCapturePhase::AfterParentHeldBeforeDescend,
                            &relative,
                        );
                        parent = attachment
                            .opened
                            .as_ref()
                            .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?
                            .try_clone()
                            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                    }
                    attachments.push(attachment);
                }
            }
        }
        paths.push(HeldTargetPath {
            attachments,
            missing,
        });
    }

    let rows = rows
        .into_iter()
        .map(|(path, state)| TargetRow { path, state })
        .collect::<Vec<_>>();
    let sha256 = digest(
        &serde_json::to_vec(&rows).map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let chain = TargetDescriptorChain {
        root_path: root.to_path_buf(),
        root: root_file,
        root_object,
        paths,
    };
    Ok(TargetCapture {
        snapshot: TargetSnapshot { sha256, rows },
        chain,
    })
}

fn insert_target_state(
    rows: &mut BTreeMap<String, TargetState>,
    path: &str,
    state: TargetState,
) -> Result<(), FitAdapterError> {
    if rows.get(path).is_some_and(|prior| prior != &state) {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    rows.insert(path.to_owned(), state);
    Ok(())
}

#[cfg(target_vendor = "apple")]
fn capture_target_attachment(
    parent: &File,
    name: &str,
    path: &str,
    require_directory: bool,
) -> Result<HeldTargetAttachment, FitAdapterError> {
    let before = named_object_at(parent, name, None)?
        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
    test_target_capture_point(TargetCapturePhase::AfterNamedBeforeOpen, path);
    let (mut opened, payload_sha256) = match before.kind {
        "directory" => (
            Some(open_target_at(
                parent,
                name,
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
            )?),
            None,
        ),
        "regular" if !require_directory => {
            let mut file = open_target_at(
                parent,
                name,
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )?;
            test_target_capture_point(TargetCapturePhase::AfterLeafHeldBeforeRead, path);
            let digest = stable_descriptor_file_digest(&mut file)?;
            (Some(file), Some(digest))
        }
        "symlink" if !require_directory => (None, Some(stable_readlink_at(parent, name)?)),
        _ if !require_directory => (None, None),
        _ => return Err(adapter_error(AdapterErrorId::TargetUnavailable)),
    };
    let object = if let Some(file) = opened.as_mut() {
        metadata_object(
            &file
                .metadata()
                .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
            payload_sha256,
        )?
    } else {
        named_object_at(parent, name, payload_sha256)?
            .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?
    };
    if exact_entry_at(parent, name)? != ExactEntry::Exact
        || named_object_at(parent, name, object.payload_sha256.clone())?.as_ref() != Some(&object)
        || !same_attachment_contract(&before, &object)
    {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(HeldTargetAttachment {
        path: path.to_owned(),
        parent: parent
            .try_clone()
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
        name: name.to_owned(),
        object,
        opened,
    })
}

impl TargetDescriptorChain {
    fn revalidate(&mut self) -> Result<(), FitAdapterError> {
        let path_root = fs::symlink_metadata(&self.root_path)
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        let opened_root = self
            .root
            .metadata()
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        if path_root.file_type().is_symlink()
            || metadata_object(&path_root, None)? != self.root_object
            || metadata_object(&opened_root, None)? != self.root_object
        {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        for path in &mut self.paths {
            for attachment in &mut path.attachments {
                let payload = match (&mut attachment.opened, attachment.object.kind) {
                    (Some(opened), "regular") => Some(stable_descriptor_file_digest(opened)?),
                    (None, "symlink") => {
                        Some(stable_readlink_at(&attachment.parent, &attachment.name)?)
                    }
                    _ => None,
                };
                let opened_matches = match &attachment.opened {
                    Some(opened) => {
                        metadata_object(
                            &opened
                                .metadata()
                                .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
                            payload.clone(),
                        )? == attachment.object
                    }
                    None => true,
                };
                if exact_entry_at(&attachment.parent, &attachment.name)? != ExactEntry::Exact
                    || !opened_matches
                    || named_object_at(&attachment.parent, &attachment.name, payload)?.as_ref()
                        != Some(&attachment.object)
                {
                    let _ = &attachment.path;
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
            }
            if let Some(missing) = &path.missing {
                if exact_entry_at(&missing.parent, &missing.name)? != ExactEntry::Absent
                    || named_object_at(&missing.parent, &missing.name, None)?.is_some()
                {
                    let _ = &missing.path;
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
            }
        }
        Ok(())
    }
}

fn same_attachment_contract(left: &ObjectRow, right: &ObjectRow) -> bool {
    left.change_version == right.change_version
        && left.kind == right.kind
        && left.device == right.device
        && left.inode == right.inode
        && left.links == right.links
        && left.uid == right.uid
        && left.gid == right.gid
        && left.mode == right.mode
        && left.byte_length == right.byte_length
}

#[cfg(target_vendor = "apple")]
fn open_target_at(parent: &File, name: &str, flags: i32) -> Result<File, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(target_vendor = "apple")]
fn named_object_at(
    parent: &File,
    name: &str,
    payload_sha256: Option<String>,
) -> Result<Option<ObjectRow>, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return if last_errno() == libc::ENOENT {
            Ok(None)
        } else {
            Err(adapter_error(AdapterErrorId::TargetUnavailable))
        };
    }
    let stat = unsafe { stat.assume_init() };
    Ok(Some(stat_object(&stat, payload_sha256)?))
}

#[cfg(target_vendor = "apple")]
fn stat_object(
    stat: &libc::stat,
    payload_sha256: Option<String>,
) -> Result<ObjectRow, FitAdapterError> {
    let mode = stat.st_mode as u32;
    let kind = match mode & libc::S_IFMT as u32 {
        value if value == libc::S_IFDIR as u32 => "directory",
        value if value == libc::S_IFREG as u32 => "regular",
        value if value == libc::S_IFLNK as u32 => "symlink",
        value if value == libc::S_IFIFO as u32 => "fifo",
        value if value == libc::S_IFSOCK as u32 => "socket",
        _ => "special",
    };
    let byte_length = u64::try_from(stat.st_size)
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    Ok(ObjectRow {
        kind,
        device: stat.st_dev as u64,
        inode: stat.st_ino,
        links: stat.st_nlink as u64,
        uid: stat.st_uid,
        gid: stat.st_gid,
        mode,
        byte_length,
        payload_sha256,
        change_version: ProtectedChangeVersion {
            ctime_seconds: stat.st_ctime,
            ctime_nanoseconds: stat.st_ctime_nsec,
        },
    })
}

fn metadata_object(
    metadata: &fs::Metadata,
    payload_sha256: Option<String>,
) -> Result<ObjectRow, FitAdapterError> {
    #[cfg(not(unix))]
    {
        let _ = (metadata, payload_sha256);
        return Err(adapter_error(AdapterErrorId::UnsupportedHost));
    }
    #[cfg(unix)]
    {
        let file_type = metadata.file_type();
        let kind = if file_type.is_symlink() {
            "symlink"
        } else if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "regular"
        } else if file_type.is_fifo() {
            "fifo"
        } else if file_type.is_socket() {
            "socket"
        } else {
            "special"
        };
        Ok(ObjectRow {
            kind,
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode(),
            byte_length: metadata.len(),
            payload_sha256,
            change_version: ProtectedChangeVersion {
                ctime_seconds: metadata.ctime(),
                ctime_nanoseconds: metadata.ctime_nsec(),
            },
        })
    }
}

#[cfg(target_vendor = "apple")]
fn stable_descriptor_file_digest(file: &mut File) -> Result<String, FitAdapterError> {
    let before = file
        .metadata()
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    if !before.is_file() || before.len() > MAX_FENCE_FILE_BYTES {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let first = bounded_file_read(file, before.len())?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let second = bounded_file_read(file, before.len())?;
    let after = file
        .metadata()
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    if first != second
        || first.len() as u64 != before.len()
        || object_identity(&before) != object_identity(&after)
    {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(digest(&first))
}

#[cfg(target_vendor = "apple")]
fn stable_readlink_at(parent: &File, name: &str) -> Result<String, FitAdapterError> {
    let first = readlink_at(parent, name)?;
    let second = readlink_at(parent, name)?;
    if first != second {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(digest(&first))
}

#[cfg(target_vendor = "apple")]
fn readlink_at(parent: &File, name: &str) -> Result<Vec<u8>, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut bytes = vec![0_u8; 64 * 1024];
    let length = unsafe {
        libc::readlinkat(
            parent.as_raw_fd(),
            name.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
        )
    };
    if length < 0 || length as usize == bytes.len() {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    bytes.truncate(length as usize);
    Ok(bytes)
}

#[cfg(target_vendor = "apple")]
fn exact_entry_at(parent: &File, name: &str) -> Result<ExactEntry, FitAdapterError> {
    let duplicated = parent
        .try_clone()
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?
        .into_raw_fd();
    let directory = unsafe { libc::fdopendir(duplicated) };
    if directory.is_null() {
        unsafe {
            libc::close(duplicated);
        }
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    unsafe {
        libc::rewinddir(directory);
    }
    let expected = name.as_bytes();
    let mut exact = false;
    let mut alias = false;
    let mut entries = 0_usize;
    let mut name_bytes = 0_usize;
    loop {
        unsafe {
            *libc::__error() = 0;
        }
        let entry = unsafe { libc::readdir(directory) };
        if entry.is_null() {
            let failed = last_errno() != 0;
            unsafe {
                libc::closedir(directory);
            }
            if failed {
                return Err(adapter_error(AdapterErrorId::TargetUnavailable));
            }
            break;
        }
        let observed = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if matches!(observed, b"." | b"..") {
            continue;
        }
        entries += 1;
        name_bytes = name_bytes.saturating_add(observed.len());
        if entries > MAX_TARGET_ENTRY_SCAN || name_bytes > MAX_TARGET_NAME_BYTES {
            unsafe {
                libc::closedir(directory);
            }
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        if observed == expected {
            exact = true;
        } else if observed.eq_ignore_ascii_case(expected) {
            alias = true;
        }
    }
    Ok(if alias {
        ExactEntry::Alias
    } else if exact {
        ExactEntry::Exact
    } else {
        ExactEntry::Absent
    })
}

#[cfg(target_vendor = "apple")]
fn last_errno() -> i32 {
    unsafe { *libc::__error() }
}

fn require_desired_target<E: RepositoryFitPermitEffects>(
    root: &Path,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    effects: &mut ScopedEffects<E>,
) -> Result<TargetSnapshot, FitAdapterError> {
    let authorized = effects.revalidate_authorized_target()?;
    let mut capture = capture_target_descriptor_chain(root, request)?;
    if capture.snapshot != authorized {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    capture.chain.revalidate()?;
    let snapshot = capture.snapshot;
    validate_managed_ancestors(&snapshot, &permit.target_prestate, request)?;
    let rows = snapshot
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let root_object = present_target_row(&rows, "")
        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
    for file in &request.desired.files {
        let object = present_target_row(&rows, file.path.as_str())
            .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
        let expected_sha256 = file.sha256();
        if object.kind != "regular"
            || object.device != root_object.device
            || object.links != 1
            || object.uid != root_object.uid
            || object.gid != root_object.gid
            || object.mode & 0o7777 != request.unix_modes[file.path.as_str()]
            || object.payload_sha256.as_deref() != Some(expected_sha256.as_str())
        {
            return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
        }
    }
    Ok(snapshot)
}

fn validate_managed_ancestors(
    current: &TargetSnapshot,
    expected: &TargetSnapshot,
    request: &OpaqueFitApplyRequest,
) -> Result<(), FitAdapterError> {
    let contract = managed_ancestor_contract(expected, request)?;
    let (_, postimage) = classify_managed_ancestor_contract(current, &contract)?;
    if postimage {
        Ok(())
    } else {
        Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid))
    }
}

fn managed_ancestor_contract(
    snapshot: &TargetSnapshot,
    request: &OpaqueFitApplyRequest,
) -> Result<ManagedAncestorContract, FitAdapterError> {
    let target_paths = request
        .desired
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    let paths = managed_ancestor_paths_for_targets(&target_paths);
    let snapshot_rows = snapshot
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::with_capacity(paths.len());
    for path in paths {
        let expectation = match snapshot_rows.get(path.as_str()) {
            Some(TargetState::Present(object)) if object.kind == "directory" => {
                ManagedAncestorExpectation::Existing {
                    device: object.device,
                    inode: object.inode,
                    uid: object.uid,
                    gid: object.gid,
                    mode: object.mode,
                }
            }
            Some(TargetState::Missing) if !path.is_empty() => ManagedAncestorExpectation::Missing,
            _ => return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid)),
        };
        rows.push(ManagedAncestorContractRow { path, expectation });
    }
    let contract = ManagedAncestorContract { rows };
    let leaf_paths = target_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    if contract.valid_for_leaf_paths(&leaf_paths) {
        Ok(contract)
    } else {
        Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid))
    }
}

fn classify_managed_ancestor_contract(
    current: &TargetSnapshot,
    contract: &ManagedAncestorContract,
) -> Result<(bool, bool), FitAdapterError> {
    let current_rows = current
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let current_root = present_target_row(&current_rows, "")
        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
    let mut preimage = true;
    let mut postimage = true;
    for row in &contract.rows {
        let current_state = current_rows
            .get(row.path.as_str())
            .copied()
            .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
        match (&row.expectation, current_state) {
            (
                ManagedAncestorExpectation::Existing {
                    device,
                    inode,
                    uid,
                    gid,
                    mode,
                },
                TargetState::Present(object),
            ) => {
                let exact = object.kind == "directory"
                    && object.device == *device
                    && object.inode == *inode
                    && object.uid == *uid
                    && object.gid == *gid
                    && object.mode == *mode;
                preimage &= exact;
                postimage &= exact;
            }
            (ManagedAncestorExpectation::Missing, TargetState::Missing) => {
                postimage = false;
            }
            (ManagedAncestorExpectation::Missing, TargetState::Present(object)) => {
                preimage = false;
                postimage &= object.valid_created_managed_ancestor(current_root);
            }
            _ => {
                preimage = false;
                postimage = false;
            }
        }
    }
    Ok((preimage, postimage))
}

pub(super) fn observe_recovery_target_contract(
    root: &Path,
    leaf_paths: &[CanonicalPath],
    contract: &ManagedAncestorContract,
) -> Result<RecoveryTargetContractObservation, FitAdapterError> {
    let leaf_path_strings = leaf_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    if !contract.valid_for_leaf_paths(&leaf_path_strings) {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    let mut capture = capture_target_descriptor_chain_for_paths(root, leaf_paths)?;
    capture.chain.revalidate()?;
    let retained_root_path = retained_descriptor_path(&capture.chain.root)?;
    let (ancestor_preimage, ancestor_postimage) =
        classify_managed_ancestor_contract(&capture.snapshot, contract)?;
    let rows = capture
        .snapshot
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let root_object = present_target_row(&rows, "")
        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
    let leaves = leaf_paths
        .iter()
        .map(|path| {
            let state = rows
                .get(path.as_str())
                .copied()
                .ok_or_else(|| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))?;
            Ok(match state {
                TargetState::Missing => RecoveryLeafObservation {
                    path: path.as_str().to_owned(),
                    payload_sha256: None,
                    mode: None,
                    valid_managed_leaf: false,
                },
                TargetState::Present(object) => RecoveryLeafObservation {
                    path: path.as_str().to_owned(),
                    payload_sha256: object.payload_sha256.clone(),
                    mode: Some(object.mode & 0o7777),
                    valid_managed_leaf: object.kind == "regular"
                        && object.device == root_object.device
                        && object.links == 1
                        && object.uid == root_object.uid
                        && object.gid == root_object.gid,
                },
            })
        })
        .collect::<Result<Vec<_>, FitAdapterError>>()?;
    capture.chain.revalidate()?;
    if retained_descriptor_path(&capture.chain.root)? != retained_root_path {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let root_binding = root_binding_from_canonical_path(
        &retained_root_path,
        root_object.device,
        root_object.inode,
    );
    Ok(RecoveryTargetContractObservation {
        root_binding,
        ancestor_preimage,
        ancestor_postimage,
        leaves,
    })
}

#[cfg(target_vendor = "apple")]
fn retained_descriptor_path(root: &File) -> Result<PathBuf, FitAdapterError> {
    let mut buffer = [0 as libc::c_char; libc::PATH_MAX as usize];
    if unsafe { libc::fcntl(root.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } < 0 {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let bytes = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_bytes();
    if bytes.first() != Some(&b'/') {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

#[cfg(not(target_vendor = "apple"))]
fn retained_descriptor_path(root: &File) -> Result<PathBuf, FitAdapterError> {
    let _ = root;
    Err(adapter_error(AdapterErrorId::UnsupportedHost))
}

fn managed_ancestor_paths(request: &OpaqueFitApplyRequest) -> BTreeSet<String> {
    let target_paths = request
        .desired
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    managed_ancestor_paths_for_targets(&target_paths)
}

fn managed_ancestor_paths_for_targets(target_paths: &[CanonicalPath]) -> BTreeSet<String> {
    let mut paths = BTreeSet::from([String::new()]);
    for path in target_paths {
        let components = path.components().collect::<Vec<_>>();
        let mut relative = String::new();
        for component in &components[..components.len() - 1] {
            if !relative.is_empty() {
                relative.push('/');
            }
            relative.push_str(component);
            paths.insert(relative.clone());
        }
    }
    paths
}

fn present_target_row<'a>(
    rows: &BTreeMap<&str, &'a TargetState>,
    path: &str,
) -> Option<&'a ObjectRow> {
    match rows.get(path) {
        Some(TargetState::Present(object)) => Some(object),
        Some(TargetState::Missing) | None => None,
    }
}

fn target_rollback_equivalent(
    current: &TargetSnapshot,
    expected: &TargetSnapshot,
    request: &OpaqueFitApplyRequest,
) -> bool {
    if current.rows.len() != expected.rows.len() {
        return false;
    }
    let leaves = request
        .desired
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<BTreeSet<_>>();
    current
        .rows
        .iter()
        .zip(&expected.rows)
        .all(|(left, right)| {
            if left.path != right.path {
                return false;
            }
            let leaf = leaves.contains(left.path.as_str());
            match (&left.state, &right.state) {
                (TargetState::Missing, TargetState::Missing) => true,
                (TargetState::Present(left_object), TargetState::Present(right_object)) => {
                    left_object.rollback_equivalent(right_object, leaf)
                }
                _ => false,
            }
        })
}

fn capture_protected(
    root: &Path,
    request: &OpaqueFitApplyRequest,
    boundary: ProtectedCaptureBoundary,
) -> Result<ProtectedSnapshot, FitAdapterError> {
    let first = collect_protected(root, request, boundary, true)?;
    test_protected_capture_point(boundary, ProtectedCapturePhase::BeforeFinalRecheck, b"");
    let final_recheck = collect_protected(root, request, boundary, false)?;
    if first != final_recheck {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(final_recheck)
}

/// A complete descriptor-relative protected-tree collect. `capture_protected`
/// accepts one only after a second complete collect returns the identical
/// versioned rows, so no row that changes after its first read can survive the
/// final recheck as the old protected state.
fn collect_protected(
    root: &Path,
    request: &OpaqueFitApplyRequest,
    boundary: ProtectedCaptureBoundary,
    run_capture_hooks: bool,
) -> Result<ProtectedSnapshot, FitAdapterError> {
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (root, request, boundary, run_capture_hooks);
        return Err(adapter_error(AdapterErrorId::UnsupportedHost));
    }
    #[cfg(target_vendor = "apple")]
    {
        let path_metadata = fs::symlink_metadata(root)
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let mut root_descriptor = options
            .open(root)
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        let root_object = metadata_object(
            &root_descriptor
                .metadata()
                .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
            None,
        )?;
        if root_object.kind != "directory" || metadata_object(&path_metadata, None)? != root_object
        {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        let allowed = request
            .desired
            .files
            .iter()
            .map(|file| file.path.as_str().as_bytes().to_vec())
            .collect::<BTreeSet<_>>();
        let mut budget = FenceBudget {
            entries: 0,
            name_bytes: 0,
            bytes: 0,
        };
        let mut visited = BTreeSet::from([(root_object.device, root_object.inode)]);
        let mut rows = Vec::new();
        visit_protected_descriptor(
            &mut root_descriptor,
            &[],
            root_object.device,
            &allowed,
            boundary,
            run_capture_hooks,
            0,
            &mut budget,
            &mut visited,
            &mut rows,
        )?;
        let final_path_metadata = fs::symlink_metadata(root)
            .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
        let final_root_object = metadata_object(
            &root_descriptor
                .metadata()
                .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?,
            None,
        )?;
        if final_path_metadata.file_type().is_symlink()
            || metadata_object(&final_path_metadata, None)? != root_object
            || final_root_object != root_object
        {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        rows.sort_by(|left, right| left.path.cmp(&right.path));
        let sha256 = digest(
            &serde_json::to_vec(&rows)
                .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        );
        Ok(ProtectedSnapshot { sha256, rows })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProtectedEnumeratedEntry {
    name: Vec<u8>,
    inode: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManagedProtectedRole {
    ExactLeaf,
    StrictAncestor,
    Protected,
}

#[cfg(target_vendor = "apple")]
#[allow(clippy::too_many_arguments)]
fn visit_protected_descriptor(
    directory: &mut File,
    prefix: &[u8],
    root_device: u64,
    allowed: &BTreeSet<Vec<u8>>,
    boundary: ProtectedCaptureBoundary,
    run_capture_hooks: bool,
    depth: usize,
    budget: &mut FenceBudget,
    visited: &mut BTreeSet<(u64, u64)>,
    rows: &mut Vec<ProtectedRow>,
) -> Result<(), FitAdapterError> {
    if depth > MAX_FENCE_DEPTH {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let entries = enumerate_protected_entries(directory)?;
    budget.entries = budget
        .entries
        .checked_add(entries.len())
        .filter(|count| *count <= MAX_FENCE_ENTRIES)
        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let observed_name_bytes = entries
        .iter()
        .try_fold(0_usize, |total, entry| total.checked_add(entry.name.len()));
    budget.name_bytes = budget
        .name_bytes
        .checked_add(
            observed_name_bytes.ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?,
        )
        .filter(|count| *count <= MAX_TARGET_NAME_BYTES)
        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;

    for entry in &entries {
        let relative = append_protected_component(prefix, &entry.name)?;
        let role = managed_protected_role(&relative, allowed)?;
        if run_capture_hooks {
            test_protected_capture_point(
                boundary,
                ProtectedCapturePhase::AfterEnumerationBeforeChildOpen,
                &relative,
            );
        }
        let before = named_versioned_object_at_bytes(directory, &entry.name, None)?
            .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
        if before.object.inode != entry.inode || before.object.device != root_device {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        match before.object.kind {
            "directory" => {
                if role == ManagedProtectedRole::ExactLeaf {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                let mut child = open_target_at_bytes(
                    directory,
                    &entry.name,
                    libc::O_RDONLY
                        | libc::O_DIRECTORY
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC
                        | libc::O_NONBLOCK,
                )?;
                let held_metadata = child
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let held = metadata_versioned_object(&held_metadata, None)?;
                if held != before || !visited.insert((held.object.device, held.object.inode)) {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                if role == ManagedProtectedRole::Protected {
                    rows.push(ProtectedRow {
                        path: relative.clone(),
                        object: held.object.clone(),
                        change_version: held.change_version,
                    });
                }
                if run_capture_hooks {
                    test_protected_capture_point(
                        boundary,
                        ProtectedCapturePhase::AfterDirectoryHeldBeforeDescend,
                        &relative,
                    );
                }
                visit_protected_descriptor(
                    &mut child,
                    &relative,
                    root_device,
                    allowed,
                    boundary,
                    run_capture_hooks,
                    depth + 1,
                    budget,
                    visited,
                    rows,
                )?;
                let held_after_metadata = child
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let held_after = metadata_versioned_object(&held_after_metadata, None)?;
                if held_after != held
                    || named_versioned_object_at_bytes(directory, &entry.name, None)?.as_ref()
                        != Some(&held)
                {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
            }
            "regular" => {
                if role == ManagedProtectedRole::StrictAncestor || before.object.links != 1 {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                if role == ManagedProtectedRole::Protected {
                    budget.bytes = budget
                        .bytes
                        .checked_add(before.object.byte_length)
                        .filter(|bytes| *bytes <= MAX_FENCE_TOTAL_BYTES)
                        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
                }
                let mut file = open_target_at_bytes(
                    directory,
                    &entry.name,
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )?;
                let opened_metadata = file
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let opened = metadata_versioned_object(&opened_metadata, None)?;
                if opened != before {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                let payload_sha256 = stable_descriptor_file_digest(&mut file)?;
                let object_metadata = file
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let object =
                    metadata_versioned_object(&object_metadata, Some(payload_sha256.clone()))?;
                if !same_versioned_attachment_contract(&before, &object)
                    || named_versioned_object_at_bytes(
                        directory,
                        &entry.name,
                        Some(payload_sha256.clone()),
                    )?
                    .as_ref()
                        != Some(&object)
                {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                let mut named = open_target_at_bytes(
                    directory,
                    &entry.name,
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )?;
                let named_before_metadata = named
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                let named_before = metadata_versioned_object(&named_before_metadata, None)?;
                let named_after_digest = stable_descriptor_file_digest(&mut named)?;
                let named_after_metadata = named
                    .metadata()
                    .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
                if !same_versioned_attachment_contract(&object, &named_before)
                    || named_after_digest != payload_sha256
                    || metadata_versioned_object(
                        &named_after_metadata,
                        Some(payload_sha256.clone()),
                    )? != object
                    || named_versioned_object_at_bytes(
                        directory,
                        &entry.name,
                        Some(payload_sha256),
                    )?
                    .as_ref()
                        != Some(&object)
                {
                    return Err(adapter_error(AdapterErrorId::TargetUnavailable));
                }
                if role == ManagedProtectedRole::Protected {
                    rows.push(ProtectedRow {
                        path: relative.clone(),
                        object: object.object,
                        change_version: object.change_version,
                    });
                    if run_capture_hooks {
                        test_protected_capture_point(
                            boundary,
                            ProtectedCapturePhase::AfterRegularRowRevalidated,
                            &relative,
                        );
                    }
                }
            }
            "symlink" | "fifo" | "socket" | "special" | _ => {
                return Err(adapter_error(AdapterErrorId::TargetUnavailable));
            }
        }
    }
    if enumerate_protected_entries(directory)? != entries {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(())
}

#[cfg(target_vendor = "apple")]
fn append_protected_component(prefix: &[u8], name: &[u8]) -> Result<Vec<u8>, FitAdapterError> {
    let required = prefix
        .len()
        .checked_add(usize::from(!prefix.is_empty()))
        .and_then(|length| length.checked_add(name.len()))
        .filter(|length| *length <= MAX_TARGET_NAME_BYTES)
        .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut relative = Vec::with_capacity(required);
    relative.extend_from_slice(prefix);
    if !prefix.is_empty() {
        relative.push(b'/');
    }
    relative.extend_from_slice(name);
    Ok(relative)
}

#[cfg(target_vendor = "apple")]
fn managed_protected_role(
    relative: &[u8],
    allowed: &BTreeSet<Vec<u8>>,
) -> Result<ManagedProtectedRole, FitAdapterError> {
    if allowed.contains(relative) {
        return Ok(ManagedProtectedRole::ExactLeaf);
    }
    if allowed
        .iter()
        .any(|target| protected_descendant(target, relative))
    {
        return Ok(ManagedProtectedRole::StrictAncestor);
    }
    let folded = ascii_fold(relative);
    if allowed.iter().any(|target| {
        let target = ascii_fold(target);
        target == folded || protected_descendant(&target, &folded)
    }) {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(ManagedProtectedRole::Protected)
}

#[cfg(target_vendor = "apple")]
fn protected_descendant(candidate: &[u8], ancestor: &[u8]) -> bool {
    candidate.len() > ancestor.len()
        && candidate.starts_with(ancestor)
        && candidate.get(ancestor.len()) == Some(&b'/')
}

#[cfg(target_vendor = "apple")]
fn ascii_fold(value: &[u8]) -> Vec<u8> {
    value.iter().map(u8::to_ascii_lowercase).collect()
}

#[cfg(target_vendor = "apple")]
fn enumerate_protected_entries(
    directory: &File,
) -> Result<Vec<ProtectedEnumeratedEntry>, FitAdapterError> {
    let duplicated = unsafe { libc::dup(directory.as_raw_fd()) };
    if duplicated < 0 {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    if unsafe { libc::fcntl(duplicated, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        unsafe {
            libc::close(duplicated);
        }
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let stream = unsafe { libc::fdopendir(duplicated) };
    if stream.is_null() {
        unsafe {
            libc::close(duplicated);
        }
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    let stream = ProtectedDirectoryStream(stream);
    unsafe {
        libc::rewinddir(stream.0);
    }
    let mut entries = Vec::new();
    let mut name_bytes = 0_usize;
    loop {
        unsafe {
            *libc::__error() = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if last_errno() != 0 {
                return Err(adapter_error(AdapterErrorId::TargetUnavailable));
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if matches!(name, b"." | b"..") {
            continue;
        }
        name_bytes = name_bytes
            .checked_add(name.len())
            .filter(|bytes| *bytes <= MAX_TARGET_NAME_BYTES)
            .ok_or_else(|| adapter_error(AdapterErrorId::TargetUnavailable))?;
        if entries.len() >= MAX_FENCE_ENTRIES || unsafe { (*entry).d_ino } == 0 {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        entries.push(ProtectedEnumeratedEntry {
            name: name.to_vec(),
            inode: unsafe { (*entry).d_ino as u64 },
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    let mut folded = BTreeSet::new();
    for entry in &entries {
        if !folded.insert(ascii_fold(&entry.name)) {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
    }
    Ok(entries)
}

#[cfg(target_vendor = "apple")]
struct ProtectedDirectoryStream(*mut libc::DIR);

#[cfg(target_vendor = "apple")]
impl Drop for ProtectedDirectoryStream {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}

#[cfg(target_vendor = "apple")]
fn open_target_at_bytes(parent: &File, name: &[u8], flags: i32) -> Result<File, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(target_vendor = "apple")]
fn named_object_at_bytes(
    parent: &File,
    name: &[u8],
    payload_sha256: Option<String>,
) -> Result<Option<ObjectRow>, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return if last_errno() == libc::ENOENT {
            Ok(None)
        } else {
            Err(adapter_error(AdapterErrorId::TargetUnavailable))
        };
    }
    let stat = unsafe { stat.assume_init() };
    Ok(Some(stat_object(&stat, payload_sha256)?))
}

#[cfg(target_vendor = "apple")]
fn named_versioned_object_at_bytes(
    parent: &File,
    name: &[u8],
    payload_sha256: Option<String>,
) -> Result<Option<VersionedProtectedObject>, FitAdapterError> {
    let name = CString::new(name).map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return if last_errno() == libc::ENOENT {
            Ok(None)
        } else {
            Err(adapter_error(AdapterErrorId::TargetUnavailable))
        };
    }
    let stat = unsafe { stat.assume_init() };
    Ok(Some(VersionedProtectedObject {
        object: stat_object(&stat, payload_sha256)?,
        change_version: ProtectedChangeVersion {
            ctime_seconds: stat.st_ctime,
            ctime_nanoseconds: stat.st_ctime_nsec,
        },
    }))
}

#[cfg(unix)]
fn metadata_versioned_object(
    metadata: &fs::Metadata,
    payload_sha256: Option<String>,
) -> Result<VersionedProtectedObject, FitAdapterError> {
    Ok(VersionedProtectedObject {
        object: metadata_object(metadata, payload_sha256)?,
        change_version: ProtectedChangeVersion {
            ctime_seconds: metadata.ctime(),
            ctime_nanoseconds: metadata.ctime_nsec(),
        },
    })
}

fn same_versioned_attachment_contract(
    left: &VersionedProtectedObject,
    right: &VersionedProtectedObject,
) -> bool {
    left.change_version == right.change_version
        && same_attachment_contract(&left.object, &right.object)
}

#[cfg(unix)]
fn bounded_file_read(file: &mut File, expected: u64) -> Result<Vec<u8>, FitAdapterError> {
    let mut bytes = Vec::with_capacity(expected.min(1024 * 1024) as usize);
    Read::by_ref(file)
        .take(MAX_FENCE_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    if bytes.len() as u64 > MAX_FENCE_FILE_BYTES {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn object_identity(metadata: &fs::Metadata) -> (u64, u64, u64, u32, u32, u32, u64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.nlink(),
        metadata.uid(),
        metadata.gid(),
        metadata.mode(),
        metadata.len(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

pub(super) fn production_authority_identity(
    authority_id: String,
) -> Result<Arc<AuthorityIdentity>, FitAdapterError> {
    if !valid_digest(&authority_id) {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    Ok(Arc::new(AuthorityIdentity { authority_id }))
}

/// Performs every deterministic request, context, target, and protected-tree
/// refusal before the authority store may be opened or initialized.
pub(super) fn prepare_production_permit(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
) -> Result<PreparedProductionPermit, FitAdapterError> {
    let protected_prestate = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::PermitIssuance,
    )?;
    revalidate_apply_request(context, request)?;
    let permit_target = capture_target_descriptor_chain(context.worktree_root(), request)?;
    let target_prestate = permit_target.snapshot.clone();
    revalidate_apply_request(context, request)?;
    let protected_recheck = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::PermitIssuanceRecheck,
    )?;
    let target_recheck = capture_target(context.worktree_root(), request)?;
    if target_recheck != target_prestate || protected_recheck != protected_prestate {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    let binding = permit_binding(request, &target_prestate, &protected_prestate)?;
    Ok(PreparedProductionPermit {
        binding,
        target_prestate,
        target_chain: permit_target.chain,
        protected_prestate,
    })
}

pub(super) fn prepare_recovery_managed_ancestor_contract(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
) -> Result<ManagedAncestorContract, FitAdapterError> {
    revalidate_apply_request(context, request)?;
    let mut capture = capture_target_descriptor_chain(context.worktree_root(), request)?;
    let snapshot = capture.snapshot.clone();
    capture.chain.revalidate()?;
    revalidate_apply_request(context, request)?;
    let recheck = capture_target(context.worktree_root(), request)?;
    if recheck != snapshot {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    managed_ancestor_contract(&snapshot, request)
}

pub(super) fn prepared_managed_ancestor_contract(
    prepared: &PreparedProductionPermit,
    request: &OpaqueFitApplyRequest,
) -> Result<ManagedAncestorContract, FitAdapterError> {
    managed_ancestor_contract(&prepared.target_prestate, request)
}

pub(super) fn production_reservation_binding(
    prepared: &PreparedProductionPermit,
    authority: &Arc<AuthorityIdentity>,
    issued_tick: u64,
    expires_tick: u64,
    nonce_sha256: String,
) -> Result<ProductionReservationBinding, FitAdapterError> {
    if !valid_digest(&nonce_sha256)
        || expires_tick < issued_tick
        || expires_tick.saturating_sub(issued_tick) > MAX_PERMIT_LIFETIME
    {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    let binding_sha256 = digest(
        &serde_json::to_vec(&("repository-fit-production-binding-v1", &prepared.binding))
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let semantic_effect_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-production-semantic-effect-v1",
            &prepared.binding.context_id,
            &prepared.binding.candidate_id,
            &prepared.binding.repository_root_id,
            &prepared.binding.worktree_root_id,
            &prepared.binding.root_binding,
            &prepared.binding.plan_record_sha256,
            &prepared.binding.plan_sha256,
            &prepared.binding.accepted_plan_sha256,
            &prepared.binding.desired_state_sha256,
            &prepared.binding.source_authority_sha256,
            &prepared.binding.target_prestate_sha256,
            &prepared.binding.allowed_mutation_set_sha256,
            &prepared.binding.rollback_policy_sha256,
            &prepared.binding.protected_prestate_sha256,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let target_scope_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-production-target-scope-v1",
            &prepared.binding.repository_root_id,
            &prepared.binding.worktree_root_id,
            &prepared.binding.root_binding,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let permit_id = permit_id(
        authority.id(),
        &prepared.binding,
        issued_tick,
        expires_tick,
        &nonce_sha256,
    )?;
    Ok(ProductionReservationBinding {
        binding_sha256,
        semantic_effect_id,
        target_scope_id,
        permit_id,
        nonce_sha256,
    })
}

/// Rechecks the complete pre-reservation state and moves the concrete effect
/// adapter into exactly one permit/lease pair. A post-reservation race can only
/// yield a terminal ledger rejection; it cannot reach the mutation kernel.
pub(super) fn activate_production_permit<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    mut prepared: PreparedProductionPermit,
    mut effects: E,
    authority: Arc<AuthorityIdentity>,
    issued_tick: u64,
    expires_tick: u64,
    nonce_sha256: String,
) -> Result<(RepositoryFitApplyPermit, RepositoryFitMutationLease<E>), FitAdapterError> {
    let reservation = production_reservation_binding(
        &prepared,
        &authority,
        issued_tick,
        expires_tick,
        nonce_sha256.clone(),
    )?;
    if reservation.binding_sha256
        != digest(
            &serde_json::to_vec(&("repository-fit-production-binding-v1", &prepared.binding))
                .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        )
    {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    prepared
        .target_chain
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::StalePlan))?;
    revalidate_apply_request(context, request)?;
    require_root_binding(&mut effects, &request.root_binding)?;
    let effects_target = capture_target_descriptor_chain(context.worktree_root(), request)?;
    let protected_recheck = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::PermitIssuanceRecheck,
    )?;
    let target_recheck = capture_target(context.worktree_root(), request)?;
    if effects_target.snapshot != prepared.target_prestate
        || target_recheck != prepared.target_prestate
        || effects_target.snapshot != target_recheck
        || protected_recheck != prepared.protected_prestate
    {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    let lease = RepositoryFitMutationLease {
        binding_id: prepared.binding.plan_record_sha256.clone(),
        authority: Arc::clone(&authority),
        seal: Arc::clone(&request.seal),
        effects: ScopedEffects::new(
            effects,
            request,
            context.worktree_root(),
            prepared.target_prestate.clone(),
            effects_target,
        ),
    };
    Ok((
        RepositoryFitApplyPermit {
            permit_id: reservation.permit_id,
            binding: prepared.binding,
            issued_tick,
            expires_tick,
            nonce_sha256,
            authority,
            seal: Arc::clone(&request.seal),
            target_prestate: prepared.target_prestate,
            target_chain: prepared.target_chain,
            protected_prestate: prepared.protected_prestate,
        },
        lease,
    ))
}

#[cfg(test)]
pub(super) struct TestRepositoryFitPermitAuthority {
    identity: Arc<AuthorityIdentity>,
    reservations: Mutex<BTreeSet<String>>,
}

#[cfg(test)]
impl TestRepositoryFitPermitAuthority {
    pub(super) fn new(secret: &[u8]) -> Result<Self, FitAdapterError> {
        if secret.len() < 32 {
            return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
        }
        let authority_id = digest(
            &serde_json::to_vec(&(AUTHORITY_DOMAIN, digest(secret)))
                .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        );
        Ok(Self {
            identity: Arc::new(AuthorityIdentity { authority_id }),
            reservations: Mutex::new(BTreeSet::new()),
        })
    }

    pub(super) fn issue<E: RepositoryFitPermitEffects>(
        &self,
        context: &LiveContext,
        request: &OpaqueFitApplyRequest,
        mut effects: E,
        issued_tick: u64,
        expires_tick: u64,
        nonce: &[u8],
    ) -> Result<(RepositoryFitApplyPermit, RepositoryFitMutationLease<E>), FitAdapterError> {
        if nonce.len() < MIN_NONCE_BYTES
            || expires_tick < issued_tick
            || expires_tick.saturating_sub(issued_tick) > MAX_PERMIT_LIFETIME
        {
            return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
        }
        let protected_prestate = capture_protected(
            context.worktree_root(),
            request,
            ProtectedCaptureBoundary::PermitIssuance,
        )?;
        revalidate_apply_request(context, request)?;
        require_root_binding(&mut effects, &request.root_binding)?;
        let permit_target = capture_target_descriptor_chain(context.worktree_root(), request)?;
        let target_prestate = permit_target.snapshot.clone();
        revalidate_apply_request(context, request)?;
        require_root_binding(&mut effects, &request.root_binding)?;
        let effects_target = capture_target_descriptor_chain(context.worktree_root(), request)?;
        let protected_recheck = capture_protected(
            context.worktree_root(),
            request,
            ProtectedCaptureBoundary::PermitIssuanceRecheck,
        )?;
        let target_recheck = capture_target(context.worktree_root(), request)?;
        if effects_target.snapshot != target_prestate
            || target_recheck != target_prestate
            || effects_target.snapshot != target_recheck
            || protected_recheck != protected_prestate
        {
            return Err(adapter_error(AdapterErrorId::StalePlan));
        }
        let binding = permit_binding(request, &target_prestate, &protected_prestate)?;
        let nonce_sha256 = digest(nonce);
        let reservation = digest(
            &serde_json::to_vec(&(
                &self.identity.authority_id,
                &binding.request_id,
                &binding.plan_record_sha256,
                request.seal.issuance(),
            ))
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        );
        let nonce_reservation = format!("nonce:{nonce_sha256}");
        let effect_reservation = format!("effect:{reservation}");
        let mut reservations = self
            .reservations
            .lock()
            .map_err(|_| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
        if reservations.contains(&nonce_reservation) || reservations.contains(&effect_reservation) {
            return Err(adapter_error(AdapterErrorId::ApplyPermitReplayed));
        }
        reservations.insert(nonce_reservation);
        reservations.insert(effect_reservation);
        drop(reservations);
        let permit_id = permit_id(
            &self.identity.authority_id,
            &binding,
            issued_tick,
            expires_tick,
            &nonce_sha256,
        )?;
        let lease = RepositoryFitMutationLease {
            binding_id: binding.plan_record_sha256.clone(),
            authority: Arc::clone(&self.identity),
            seal: Arc::clone(&request.seal),
            effects: ScopedEffects::new(
                effects,
                request,
                context.worktree_root(),
                target_prestate.clone(),
                effects_target,
            ),
        };
        Ok((
            RepositoryFitApplyPermit {
                permit_id,
                binding,
                issued_tick,
                expires_tick,
                nonce_sha256,
                authority: Arc::clone(&self.identity),
                seal: Arc::clone(&request.seal),
                target_prestate,
                target_chain: permit_target.chain,
                protected_prestate,
            },
            lease,
        ))
    }
}

#[cfg(test)]
pub(super) fn duplicate_authorization_for_test<E: RepositoryFitPermitEffects>(
    permit: &RepositoryFitApplyPermit,
    request: &OpaqueFitApplyRequest,
    effects: E,
) -> (RepositoryFitApplyPermit, RepositoryFitMutationLease<E>) {
    let permit_target = capture_target_descriptor_chain(&permit.target_chain.root_path, request)
        .expect("duplicate test authority requires the exact live target chain");
    assert_eq!(permit_target.snapshot, permit.target_prestate);
    let lease_target = capture_target_descriptor_chain(&permit.target_chain.root_path, request)
        .expect("duplicate test lease requires the exact live target chain");
    assert_eq!(lease_target.snapshot, permit.target_prestate);
    (
        RepositoryFitApplyPermit {
            permit_id: permit.permit_id.clone(),
            binding: permit.binding.clone(),
            issued_tick: permit.issued_tick,
            expires_tick: permit.expires_tick,
            nonce_sha256: permit.nonce_sha256.clone(),
            authority: Arc::clone(&permit.authority),
            seal: Arc::clone(&permit.seal),
            target_prestate: permit.target_prestate.clone(),
            target_chain: permit_target.chain,
            protected_prestate: permit.protected_prestate.clone(),
        },
        RepositoryFitMutationLease {
            binding_id: permit.binding.plan_record_sha256.clone(),
            authority: Arc::clone(&permit.authority),
            seal: Arc::clone(&permit.seal),
            effects: ScopedEffects::new(
                effects,
                request,
                &permit.target_chain.root_path,
                permit.target_prestate.clone(),
                lease_target,
            ),
        },
    )
}

#[cfg(test)]
pub(super) fn permit_seal_stage_for_test(request: &OpaqueFitApplyRequest) -> u8 {
    request.seal.stage_for_test()
}

#[cfg(test)]
pub(super) fn before_final_green_observation_for_test(action: impl FnOnce() + 'static) {
    BEFORE_FINAL_GREEN_OBSERVATION.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(
            prior.is_none(),
            "a final-green test action is already armed"
        );
    });
}

#[cfg(test)]
pub(super) fn before_postflight_observation_for_test(action: impl FnOnce() + 'static) {
    BEFORE_POSTFLIGHT_OBSERVATION.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(prior.is_none(), "a postflight test action is already armed");
    });
}

#[cfg(test)]
pub(super) fn target_capture_hook_for_test(
    phase: TargetCapturePhase,
    path: impl Into<String>,
    action: impl FnOnce() + 'static,
) {
    TARGET_CAPTURE_HOOK.with(|slot| {
        let prior = slot.borrow_mut().replace(TargetCaptureHook {
            phase,
            path: path.into(),
            action: Box::new(action),
        });
        assert!(
            prior.is_none(),
            "a target-capture test hook is already armed"
        );
    });
}

#[cfg(test)]
pub(super) fn assert_target_capture_hook_consumed_for_test() {
    TARGET_CAPTURE_HOOK.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "the armed target-capture test hook was not reached"
        );
    });
}

#[cfg(test)]
pub(super) fn protected_capture_hook_for_test(
    boundary: ProtectedCaptureBoundary,
    phase: ProtectedCapturePhase,
    path: impl Into<Vec<u8>>,
    action: impl FnOnce() + 'static,
) {
    PROTECTED_CAPTURE_HOOK.with(|slot| {
        let prior = slot.borrow_mut().replace(ProtectedCaptureHook {
            boundary,
            phase,
            path: path.into(),
            action: Box::new(action),
        });
        assert!(
            prior.is_none(),
            "a protected-capture test hook is already armed"
        );
    });
}

#[cfg(test)]
pub(super) fn assert_protected_capture_hook_consumed_for_test() {
    PROTECTED_CAPTURE_HOOK.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "the armed protected-capture test hook was not reached"
        );
    });
}

#[cfg(test)]
pub(super) fn reconciliation_target_hook_for_test(
    phase: ReconciliationTargetPhase,
    action: impl FnOnce() + 'static,
) {
    RECONCILIATION_TARGET_HOOK.with(|slot| {
        let prior = slot.borrow_mut().replace(ReconciliationTargetHook {
            phase,
            action: Box::new(action),
        });
        assert!(
            prior.is_none(),
            "a reconciliation-target test hook is already armed"
        );
    });
}

#[cfg(test)]
pub(super) fn assert_reconciliation_target_hook_consumed_for_test() {
    RECONCILIATION_TARGET_HOOK.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "the armed reconciliation-target test hook was not reached"
        );
    });
}

#[cfg(test)]
pub(super) fn scope_violation_for_test<E: RepositoryFitPermitEffects>(
    root: &Path,
    request: &OpaqueFitApplyRequest,
    effects: E,
) -> (FitErrorId, bool) {
    let target = capture_target_descriptor_chain(root, request).unwrap();
    let target_prestate = target.snapshot.clone();
    let mut scoped = ScopedEffects::new(effects, request, root, target_prestate, target);
    let path = CanonicalPath::parse("private/undeclared-scope-probe")
        .expect("fixed test probe path is canonical");
    let failure = scoped
        .compare_exchange(&path, &ExpectedContent::Absent, Some(b"probe"))
        .expect_err("an undeclared test mutation must be refused");
    (failure.id(), scoped.scope_violation())
}
