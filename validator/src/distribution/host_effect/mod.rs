//! Root-owned authority boundary for supported-host distribution effects.
//!
//! This module freezes the typed authority and durable-ledger interfaces. It
//! deliberately supplies no live executor, no host adapter, and no public
//! construction route. Platform adapters remain separate reviewed work, and a
//! platform that cannot execute a retained descriptor must fail as unsupported
//! before reservation, spawn, or host mutation.

mod authority;
mod ledger;

pub(crate) use authority::{
    HostEffectAuthority, HostEffectAuthorityError, HostEffectAuthorityErrorId, HostEffectDecision,
    HostEffectPermit, HostEffectPermitBinding,
};
pub(crate) use ledger::FileHostEffectLedger;

use super::HostCommandPlan;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};

const MAX_PINNED_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectState {
    Reserved,
    InFlight,
    Settled,
    Failed,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectLedgerHead {
    generation: u64,
    head_sha256: String,
}

impl HostEffectLedgerHead {
    pub(in crate::distribution::host_effect) fn new(
        generation: u64,
        head_sha256: String,
    ) -> Result<Self, HostEffectLedgerError> {
        if !is_digest(&head_sha256) {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self {
            generation,
            head_sha256,
        })
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn head_sha256(&self) -> &str {
        &self.head_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectReservation {
    issuer_id: String,
    ledger_id: String,
    key_id: String,
    permit_id: String,
    semantic_key_sha256: String,
    nonce_sha256: String,
    binding_sha256: String,
    expected_head_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
}

impl HostEffectReservation {
    pub(in crate::distribution::host_effect) fn from_permit(permit: &HostEffectPermit) -> Self {
        Self {
            issuer_id: permit.issuer_id().to_owned(),
            ledger_id: permit.ledger_id().to_owned(),
            key_id: permit.key_id().to_owned(),
            permit_id: permit.permit_id().to_owned(),
            semantic_key_sha256: permit.semantic_key_sha256().to_owned(),
            nonce_sha256: permit.nonce_sha256().to_owned(),
            binding_sha256: permit.binding_sha256().to_owned(),
            expected_head_sha256: permit.binding().expected_head_sha256.clone(),
            issued_at_unix_ms: permit.binding().issued_at_unix_ms,
            expires_at_unix_ms: permit.binding().expires_at_unix_ms,
        }
    }

    pub(crate) fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub(crate) fn issuer_id(&self) -> &str {
        &self.issuer_id
    }

    pub(crate) fn ledger_id(&self) -> &str {
        &self.ledger_id
    }

    pub(crate) fn key_id(&self) -> &str {
        &self.key_id
    }

    pub(crate) fn semantic_key_sha256(&self) -> &str {
        &self.semantic_key_sha256
    }

    pub(crate) fn expected_head_sha256(&self) -> &str {
        &self.expected_head_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectLedgerRecord {
    reservation: HostEffectReservation,
    state: HostEffectState,
    record_sha256: String,
    prior_head: HostEffectLedgerHead,
    current_head: HostEffectLedgerHead,
    outcome_sha256: Option<String>,
}

impl HostEffectLedgerRecord {
    pub(crate) fn reservation(&self) -> &HostEffectReservation {
        &self.reservation
    }

    pub(crate) const fn state(&self) -> HostEffectState {
        self.state
    }

    pub(crate) fn current_head(&self) -> &HostEffectLedgerHead {
        &self.current_head
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectTransition {
    permit_id: String,
    expected_state: HostEffectState,
    next_state: HostEffectState,
    expected_head: HostEffectLedgerHead,
    outcome_sha256: Option<String>,
}

impl HostEffectTransition {
    pub(in crate::distribution::host_effect) fn new(
        permit_id: String,
        expected_state: HostEffectState,
        next_state: HostEffectState,
        expected_head: HostEffectLedgerHead,
        outcome_sha256: Option<String>,
    ) -> Result<Self, HostEffectLedgerError> {
        let requires_outcome = matches!(
            next_state,
            HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
        );
        if !is_digest(&permit_id)
            || outcome_sha256.as_ref().is_some_and(|row| !is_digest(row))
            || outcome_sha256.is_some() != requires_outcome
            || !allowed_transition(expected_state, next_state)
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidTransition,
            ));
        }
        Ok(Self {
            permit_id,
            expected_state,
            next_state,
            expected_head,
            outcome_sha256,
        })
    }
}

/// Cross-process implementations must reserve nonce and semantic key together,
/// publish transitions with compare-and-swap head semantics, fsync before
/// acknowledgement, and classify any uncertain started effect as Ambiguous.
pub(crate) trait DurableHostEffectLedger: Send + Sync {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError>;
    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError>;
    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError>;
    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostEffectLedgerErrorId {
    InvalidRecord,
    InvalidTransition,
    StaleHead,
    Replay,
    Tampered,
    Io,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HostEffectLedgerError {
    id: HostEffectLedgerErrorId,
}

impl HostEffectLedgerError {
    pub(in crate::distribution::host_effect) const fn new(id: HostEffectLedgerErrorId) -> Self {
        Self { id }
    }

    pub(crate) const fn id(&self) -> HostEffectLedgerErrorId {
        self.id
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PinnedHostExecutableIdentity {
    canonical_path: String,
    content_sha256: String,
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    hard_links: u64,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl PinnedHostExecutableIdentity {
    pub(crate) fn binding_sha256(&self) -> Result<String, HostEffectLedgerError> {
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            identity: &'a PinnedHostExecutableIdentity,
        }
        serde_json::to_vec(&Binding {
            schema: "harness-ultragoal.pinned-host-executable.v1",
            identity: self,
        })
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord))
    }
}

/// Owns the open executable object. It is intentionally neither Clone nor
/// serializable; an executor must keep this object alive through child start.
pub(crate) struct PinnedHostExecutable {
    file: File,
    identity: PinnedHostExecutableIdentity,
}

impl PinnedHostExecutable {
    pub(in crate::distribution::host_effect) fn pin(
        path: &Path,
    ) -> Result<Self, HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let canonical = fs::canonicalize(path).map_err(|_| ledger_io())?;
            if !canonical.is_absolute() {
                return Err(invalid_record());
            }
            let mut options = OpenOptions::new();
            options
                .read(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
            let file = options.open(&canonical).map_err(|_| ledger_io())?;
            let identity = capture_executable_identity(&file, &canonical)?;
            Ok(Self { file, identity })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(invalid_record())
        }
    }

    pub(in crate::distribution::host_effect) fn file(&self) -> &File {
        &self.file
    }

    pub(crate) fn identity(&self) -> &PinnedHostExecutableIdentity {
        &self.identity
    }

    pub(in crate::distribution::host_effect) fn revalidate(
        &self,
    ) -> Result<(), HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let current =
                capture_executable_identity(&self.file, Path::new(&self.identity.canonical_path))?;
            if current != self.identity {
                return Err(HostEffectLedgerError::new(
                    HostEffectLedgerErrorId::Tampered,
                ));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err(invalid_record())
        }
    }
}

#[cfg(unix)]
fn capture_executable_identity(
    file: &File,
    canonical_path: &Path,
) -> Result<PinnedHostExecutableIdentity, HostEffectLedgerError> {
    let path_before = fs::symlink_metadata(canonical_path).map_err(|_| ledger_io())?;
    let descriptor_before = file.metadata().map_err(|_| ledger_io())?;
    validate_executable_metadata(&path_before)?;
    validate_executable_metadata(&descriptor_before)?;
    if !same_executable_object(&path_before, &descriptor_before) {
        return Err(tampered());
    }

    let mut hasher = Sha256::new();
    let mut offset = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    while offset < descriptor_before.len() {
        let remaining = (descriptor_before.len() - offset).min(buffer.len() as u64) as usize;
        let count = read_at_retry(file, &mut buffer[..remaining], offset)?;
        if count == 0 {
            return Err(tampered());
        }
        hasher.update(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(invalid_record)?;
    }
    let mut extra = [0_u8; 1];
    if read_at_retry(file, &mut extra, descriptor_before.len())? != 0 {
        return Err(tampered());
    }

    let descriptor_after = file.metadata().map_err(|_| ledger_io())?;
    let path_after = fs::symlink_metadata(canonical_path).map_err(|_| ledger_io())?;
    validate_executable_metadata(&descriptor_after)?;
    validate_executable_metadata(&path_after)?;
    if !same_executable_object(&descriptor_before, &descriptor_after)
        || !same_executable_object(&descriptor_after, &path_after)
        || fs::canonicalize(canonical_path).map_err(|_| ledger_io())? != canonical_path
    {
        return Err(tampered());
    }
    let canonical_path = canonical_path
        .to_str()
        .ok_or_else(invalid_record)?
        .to_owned();
    Ok(PinnedHostExecutableIdentity {
        canonical_path,
        content_sha256: format!("sha256:{:x}", hasher.finalize()),
        device: descriptor_after.dev(),
        inode: descriptor_after.ino(),
        mode: descriptor_after.mode(),
        uid: descriptor_after.uid(),
        gid: descriptor_after.gid(),
        hard_links: descriptor_after.nlink(),
        size: descriptor_after.len(),
        modified_seconds: descriptor_after.mtime(),
        modified_nanoseconds: descriptor_after.mtime_nsec(),
        changed_seconds: descriptor_after.ctime(),
        changed_nanoseconds: descriptor_after.ctime_nsec(),
    })
}

#[cfg(unix)]
fn validate_executable_metadata(metadata: &fs::Metadata) -> Result<(), HostEffectLedgerError> {
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_PINNED_EXECUTABLE_BYTES
        || metadata.nlink() != 1
        || metadata.mode() & 0o111 == 0
    {
        return Err(invalid_record());
    }
    Ok(())
}

#[cfg(unix)]
fn same_executable_object(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.mode() == right.mode()
        && left.uid() == right.uid()
        && left.gid() == right.gid()
        && left.nlink() == right.nlink()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(unix)]
fn read_at_retry(
    file: &File,
    buffer: &mut [u8],
    offset: u64,
) -> Result<usize, HostEffectLedgerError> {
    loop {
        match file.read_at(buffer, offset) {
            Ok(count) => return Ok(count),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ledger_io()),
        }
    }
}

fn invalid_record() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord)
}

fn tampered() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Tampered)
}

fn ledger_io() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Io)
}

/// A ledger-reserved effect. Construction remains confined to reviewed
/// host-effect descendants and must occur only after permit verification and a
/// successful Reserved -> InFlight transition.
pub(crate) struct AuthorizedHostEffect {
    permit: HostEffectPermit,
    record: HostEffectLedgerRecord,
    executable: PinnedHostExecutable,
    plan: HostCommandPlan,
}

impl AuthorizedHostEffect {
    pub(in crate::distribution::host_effect) fn new(
        permit: HostEffectPermit,
        record: HostEffectLedgerRecord,
        executable: PinnedHostExecutable,
        plan: HostCommandPlan,
    ) -> Result<Self, HostEffectLedgerError> {
        if record.state != HostEffectState::InFlight
            || record.reservation.permit_id != permit.permit_id()
            || plan.plan_sha256() != permit.binding().command_plan_sha256
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self {
            permit,
            record,
            executable,
            plan,
        })
    }

    pub(in crate::distribution::host_effect) fn permit(&self) -> &HostEffectPermit {
        &self.permit
    }

    pub(in crate::distribution::host_effect) fn record(&self) -> &HostEffectLedgerRecord {
        &self.record
    }

    pub(in crate::distribution::host_effect) fn executable(&self) -> &PinnedHostExecutable {
        &self.executable
    }

    pub(in crate::distribution::host_effect) fn plan(&self) -> &HostCommandPlan {
        &self.plan
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectOutcome {
    permit_id: String,
    state: HostEffectState,
    command_output_sha256: Vec<String>,
    observed_post_state_sha256: Option<String>,
    outcome_sha256: String,
    completed_at_unix_ms: u64,
}

impl HostEffectOutcome {
    pub(in crate::distribution::host_effect) fn new(
        permit_id: String,
        state: HostEffectState,
        command_output_sha256: Vec<String>,
        observed_post_state_sha256: Option<String>,
        outcome_sha256: String,
        completed_at_unix_ms: u64,
    ) -> Result<Self, HostEffectLedgerError> {
        if !is_digest(&permit_id)
            || !matches!(
                state,
                HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
            )
            || command_output_sha256.iter().any(|row| !is_digest(row))
            || observed_post_state_sha256
                .as_ref()
                .is_some_and(|row| !is_digest(row))
            || !is_digest(&outcome_sha256)
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self {
            permit_id,
            state,
            command_output_sha256,
            observed_post_state_sha256,
            outcome_sha256,
            completed_at_unix_ms,
        })
    }
}

fn allowed_transition(expected: HostEffectState, next: HostEffectState) -> bool {
    matches!(
        (expected, next),
        (HostEffectState::Reserved, HostEffectState::InFlight)
            | (HostEffectState::Reserved, HostEffectState::Failed)
            | (HostEffectState::InFlight, HostEffectState::Settled)
            | (HostEffectState::InFlight, HostEffectState::Failed)
            | (HostEffectState::InFlight, HostEffectState::Ambiguous)
            | (HostEffectState::Ambiguous, HostEffectState::Settled)
            | (HostEffectState::Ambiguous, HostEffectState::Failed)
    )
}

fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::sync::atomic::{AtomicU64, Ordering};

    fn d(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    #[test]
    fn ledger_transition_graph_forbids_retry_and_terminal_revival() {
        let head = HostEffectLedgerHead::new(1, d('1')).unwrap();
        for (from, to) in [
            (HostEffectState::Reserved, HostEffectState::InFlight),
            (HostEffectState::Reserved, HostEffectState::Failed),
            (HostEffectState::InFlight, HostEffectState::Settled),
            (HostEffectState::InFlight, HostEffectState::Failed),
            (HostEffectState::InFlight, HostEffectState::Ambiguous),
            (HostEffectState::Ambiguous, HostEffectState::Settled),
            (HostEffectState::Ambiguous, HostEffectState::Failed),
        ] {
            let outcome = (to != HostEffectState::InFlight).then(|| d('3'));
            HostEffectTransition::new(d('2'), from, to, head.clone(), outcome).unwrap();
        }
        for (from, to) in [
            (HostEffectState::InFlight, HostEffectState::Reserved),
            (HostEffectState::Ambiguous, HostEffectState::InFlight),
            (HostEffectState::Settled, HostEffectState::InFlight),
            (HostEffectState::Failed, HostEffectState::InFlight),
            (HostEffectState::Settled, HostEffectState::Reserved),
        ] {
            assert_eq!(
                HostEffectTransition::new(d('2'), from, to, head.clone(), None)
                    .unwrap_err()
                    .id(),
                HostEffectLedgerErrorId::InvalidTransition
            );
        }
        assert_eq!(
            HostEffectTransition::new(
                d('2'),
                HostEffectState::Reserved,
                HostEffectState::InFlight,
                head.clone(),
                Some(d('3')),
            )
            .unwrap_err()
            .id(),
            HostEffectLedgerErrorId::InvalidTransition
        );
        assert_eq!(
            HostEffectTransition::new(
                d('2'),
                HostEffectState::InFlight,
                HostEffectState::Settled,
                head,
                None,
            )
            .unwrap_err()
            .id(),
            HostEffectLedgerErrorId::InvalidTransition
        );
    }

    #[cfg(unix)]
    #[test]
    fn pinned_executable_revalidates_exact_object_and_content() {
        let fixture = ExecutableFixture::new(b"#!/bin/sh\nexit 0\n");
        let pinned = PinnedHostExecutable::pin(&fixture.path).unwrap();
        assert!(is_digest(&pinned.identity().binding_sha256().unwrap()));
        pinned.revalidate().unwrap();

        let mut changed = OpenOptions::new().write(true).open(&fixture.path).unwrap();
        changed.write_all(b"#!/bin/sh\nexit 9\n").unwrap();
        changed.sync_all().unwrap();
        assert_eq!(
            pinned.revalidate().unwrap_err().id(),
            HostEffectLedgerErrorId::Tampered
        );
    }

    #[cfg(unix)]
    #[test]
    fn pinned_executable_rejects_named_replacement_and_hardlinks() {
        let fixture = ExecutableFixture::new(b"#!/bin/sh\nexit 0\n");
        let pinned = PinnedHostExecutable::pin(&fixture.path).unwrap();
        let held = fixture.root.join("held");
        fs::rename(&fixture.path, &held).unwrap();
        fixture.write_executable(&fixture.path, b"#!/bin/sh\nexit 1\n");
        assert_eq!(
            pinned.revalidate().unwrap_err().id(),
            HostEffectLedgerErrorId::Tampered
        );

        let hardlink = fixture.root.join("hardlink");
        fs::hard_link(&fixture.path, &hardlink).unwrap();
        let error = match PinnedHostExecutable::pin(&fixture.path) {
            Ok(_) => panic!("hard-linked executable unexpectedly pinned"),
            Err(error) => error,
        };
        assert_eq!(error.id(), HostEffectLedgerErrorId::InvalidRecord);
    }

    #[cfg(unix)]
    struct ExecutableFixture {
        root: std::path::PathBuf,
        path: std::path::PathBuf,
    }

    #[cfg(unix)]
    impl ExecutableFixture {
        fn new(bytes: &[u8]) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let base = std::env::temp_dir();
            let root = loop {
                let attempt = NEXT.fetch_add(1, Ordering::Relaxed);
                let candidate = base.join(format!(
                    "hul-host-effect-pin-{}-{attempt}",
                    std::process::id()
                ));
                match fs::create_dir(&candidate) {
                    Ok(()) => break candidate,
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create executable fixture: {error}"),
                }
            };
            let path = root.join("executable");
            let fixture = Self { root, path };
            fixture.write_executable(&fixture.path, bytes);
            fixture
        }

        fn write_executable(&self, path: &Path, bytes: &[u8]) {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .unwrap();
            file.write_all(bytes).unwrap();
            file.sync_all().unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }

    #[cfg(unix)]
    impl Drop for ExecutableFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
