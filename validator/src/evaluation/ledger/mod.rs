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

include!("state_name.rs");

include!("current_snapshot.rs");

include!("initialization/file_lifecycle.rs");

include!("initialization/interruption_hooks.rs");

include!("initialization/anchor.rs");

include!("initialization/state_recovery.rs");

include!("initialization/ledger_access.rs");

include!("file/recovery_requirement.rs");

include!("file/state_transition.rs");

include!("file/published_current_requirement.rs");

include!("test_publication_pause.rs");

include!("read/current.rs");

include!("verify_anchor_record.rs");

include!("read/directory_names.rs");

include!("pending_generation.rs");

include!("publish_file.rs");

include!("owner.rs");

#[cfg(test)]
#[derive(Debug)]
pub(super) struct TestFileEvaluationExecutionLedger(FileEvaluationExecutionLedger);

#[cfg(test)]
impl TestFileEvaluationExecutionLedger {
    pub(super) fn initialize(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: EvaluationExecutionBinding,
    ) -> Result<Self, EvaluationLedgerError> {
        FileEvaluationExecutionLedger::initialize(root, key, binding).map(Self)
    }

    pub(super) fn open(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: EvaluationExecutionBinding,
    ) -> Result<Self, EvaluationLedgerError> {
        FileEvaluationExecutionLedger::open(root, key, binding).map(Self)
    }

    pub(super) fn inspect(&self) -> Result<EvaluationLedgerState, EvaluationLedgerError> {
        self.0.inspect()
    }

    pub(super) fn reserve(&mut self) -> Result<(), EvaluationLedgerError> {
        self.0.reserve()
    }

    pub(super) fn reserve_outcome(
        &mut self,
        reservation_id: &str,
    ) -> Result<ExecutionReservationOutcome, EvaluationLedgerError> {
        self.0.reserve_outcome(reservation_id)
    }

    pub(super) fn publish_result(
        &mut self,
        run_sha256: impl Into<String>,
        artifact_set_sha256: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        self.0.publish_result(run_sha256, artifact_set_sha256)
    }

    pub(super) fn mark_interrupted(
        &mut self,
        causal_code: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        self.0.mark_interrupted(causal_code)
    }

    pub(super) fn require_recovery(
        &mut self,
        causal_code: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        self.0.require_recovery(causal_code)
    }

    pub(super) fn reconcile_authenticated_publication(
        &mut self,
    ) -> Result<(), EvaluationLedgerError> {
        self.0.reconcile_authenticated_publication()
    }

    pub(super) fn complete(&mut self) -> Result<(), EvaluationLedgerError> {
        self.0.complete()
    }

    pub(super) fn terminal_proof(&self) -> Result<ExecutionTerminalProof, EvaluationLedgerError> {
        self.0.terminal_proof()
    }

    pub(super) fn set_test_initialization_interruption(root: PathBuf, stage: &'static str) {
        FileEvaluationExecutionLedger::set_test_initialization_interruption(root, stage)
    }

    pub(super) fn set_test_publication_pause(root: PathBuf, milliseconds: u64) {
        FileEvaluationExecutionLedger::set_test_publication_pause(root, milliseconds)
    }

    pub(super) fn test_publication_is_paused(root: &Path) -> bool {
        FileEvaluationExecutionLedger::test_publication_is_paused(root)
    }

    pub(super) fn release_test_publication(root: &Path) {
        FileEvaluationExecutionLedger::release_test_publication(root)
    }

    pub(super) fn set_test_final_validation_pause(root: PathBuf, milliseconds: u64) {
        FileEvaluationExecutionLedger::set_test_final_validation_pause(root, milliseconds)
    }

    pub(super) fn test_final_validation_is_paused(root: &Path) -> bool {
        FileEvaluationExecutionLedger::test_final_validation_is_paused(root)
    }

    pub(super) fn release_test_final_validation(root: &Path) {
        FileEvaluationExecutionLedger::release_test_final_validation(root)
    }

    pub(super) fn set_test_directory_scan_pause(root: PathBuf, milliseconds: u64) {
        FileEvaluationExecutionLedger::set_test_directory_scan_pause(root, milliseconds)
    }

    pub(super) fn test_directory_scan_is_paused(root: &Path) -> bool {
        FileEvaluationExecutionLedger::test_directory_scan_is_paused(root)
    }

    pub(super) fn release_test_directory_scan(root: &Path) {
        FileEvaluationExecutionLedger::release_test_directory_scan(root)
    }
}
