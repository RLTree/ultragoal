use serde::Serialize;
use std::collections::BTreeSet;

use super::digest::{digest_of, framed, valid};
use super::local::CompleteCapture;
use super::{RepoPath, RoutineBinding, RoutineError, RoutineErrorId};

const CHANGE_ROW_LIMIT: usize = 100_000;
const COMPLETE_CAPTURE_DOMAIN: &[u8] = b"routine-complete-capture-v1";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Modified,
    Added,
    Untracked,
    Renamed,
    Deleted,
    Conflict,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DirtyChange {
    path: RepoPath,
    previous_path: Option<RepoPath>,
    kind: ChangeKind,
    content_sha256: Option<String>,
}

impl DirtyChange {
    pub(crate) fn new(
        path: RepoPath,
        previous_path: Option<RepoPath>,
        kind: ChangeKind,
        content_sha256: Option<String>,
    ) -> Result<Self, RoutineError> {
        if matches!(kind, ChangeKind::Renamed) != previous_path.is_some() {
            return Err(RoutineError::new(
                RoutineErrorId::InvalidSnapshot,
                "rename-origin-cardinality-invalid",
                None,
            ));
        }
        let needs_content = !matches!(kind, ChangeKind::Deleted | ChangeKind::Conflict);
        if needs_content != content_sha256.is_some()
            || content_sha256
                .as_deref()
                .is_some_and(|digest| !valid(digest))
        {
            return Err(RoutineError::new(
                RoutineErrorId::InvalidSnapshot,
                "change-content-identity-invalid",
                None,
            ));
        }
        if previous_path
            .as_ref()
            .is_some_and(|prior| prior.case_key() == path.case_key())
        {
            return Err(RoutineError::new(
                RoutineErrorId::InvalidSnapshot,
                "rename-origin-aliases-destination",
                None,
            ));
        }
        Ok(Self {
            path,
            previous_path,
            kind,
            content_sha256,
        })
    }

    pub fn path(&self) -> &RepoPath {
        &self.path
    }
    pub fn previous_path(&self) -> Option<&RepoPath> {
        self.previous_path.as_ref()
    }
    pub fn kind(&self) -> ChangeKind {
        self.kind
    }
    pub fn content_sha256(&self) -> Option<&str> {
        self.content_sha256.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DirtySnapshot {
    snapshot_id: String,
    binding: RoutineBinding,
    status_sha256: String,
    changes: Vec<DirtyChange>,
    #[serde(skip)]
    capture_provenance: CaptureProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureProvenance(String);

#[derive(Serialize)]
struct SnapshotPayload<'a> {
    binding_id: &'a str,
    status_sha256: &'a str,
    changes: &'a [DirtyChange],
}

impl DirtySnapshot {
    fn new(
        binding: RoutineBinding,
        status_sha256: String,
        mut changes: Vec<DirtyChange>,
    ) -> Result<Self, RoutineError> {
        normalize_changes(&binding, &status_sha256, &mut changes)?;
        let snapshot_id = snapshot_identity(&binding, &status_sha256, &changes)?;
        Ok(Self {
            capture_provenance: CaptureProvenance::mint(&snapshot_id),
            snapshot_id,
            binding,
            status_sha256,
            changes,
        })
    }

    pub(super) fn from_complete_capture(capture: CompleteCapture) -> Result<Self, RoutineError> {
        let (binding, status_sha256, changes) = capture.into_parts();
        Self::new(binding, status_sha256, changes)
    }

    pub(super) fn require_complete_capture(&self) -> Result<(), RoutineError> {
        let expected = snapshot_identity(&self.binding, &self.status_sha256, &self.changes)?;
        if self.snapshot_id != expected || !self.capture_provenance.matches(&expected) {
            return Err(snapshot_error("snapshot-capture-provenance-invalid", None));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn test_forged_subset_with_recomputed_identity(&self) -> Self {
        assert!(self.changes.len() > 1, "subset fixture needs two changes");
        let mut forged = self.clone();
        forged.changes.pop();
        forged.snapshot_id =
            snapshot_identity(&forged.binding, &forged.status_sha256, &forged.changes)
                .expect("forged structural identity");
        forged
    }

    pub fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }
    pub fn binding(&self) -> &RoutineBinding {
        &self.binding
    }
    pub fn status_sha256(&self) -> &str {
        &self.status_sha256
    }
    pub fn changes(&self) -> &[DirtyChange] {
        &self.changes
    }
    pub fn is_clean(&self) -> bool {
        self.changes.is_empty()
    }
}

impl CaptureProvenance {
    fn mint(snapshot_id: &str) -> Self {
        Self(framed(&[COMPLETE_CAPTURE_DOMAIN, snapshot_id.as_bytes()]))
    }

    fn matches(&self, snapshot_id: &str) -> bool {
        self.0 == framed(&[COMPLETE_CAPTURE_DOMAIN, snapshot_id.as_bytes()])
    }
}

fn normalize_changes(
    binding: &RoutineBinding,
    status_sha256: &str,
    changes: &mut Vec<DirtyChange>,
) -> Result<(), RoutineError> {
    if changes.len() > CHANGE_ROW_LIMIT {
        return Err(RoutineError::new(
            RoutineErrorId::CaptureLimit,
            "dirty-change-row-limit-exceeded",
            None,
        ));
    }
    if !valid(status_sha256) {
        return Err(snapshot_error("status-identity-invalid", None));
    }
    changes.sort();
    validate_change_paths(changes)?;
    if binding.dirty() != !changes.is_empty() {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "candidate-dirty-state-disagrees-with-snapshot",
            None,
        ));
    }
    Ok(())
}

fn validate_change_paths(changes: &[DirtyChange]) -> Result<(), RoutineError> {
    let mut seen = BTreeSet::new();
    let mut case_keys = BTreeSet::new();
    let mut referenced = BTreeSet::new();
    for change in changes {
        if !seen.insert(change.path().clone()) {
            return Err(snapshot_error(
                "duplicate-or-conflicting-change-row",
                Some(change.path().as_str().as_bytes()),
            ));
        }
        if !case_keys.insert(change.path().case_key()) {
            return Err(snapshot_error(
                "case-alias-change-row",
                Some(change.path().as_str().as_bytes()),
            ));
        }
        if !referenced.insert(change.path().case_key())
            || change
                .previous_path()
                .is_some_and(|path| !referenced.insert(path.case_key()))
        {
            return Err(snapshot_error("ambiguous-change-path-reference", None));
        }
    }
    Ok(())
}

fn snapshot_identity(
    binding: &RoutineBinding,
    status_sha256: &str,
    changes: &[DirtyChange],
) -> Result<String, RoutineError> {
    digest_of(&SnapshotPayload {
        binding_id: binding.binding_id(),
        status_sha256,
        changes,
    })
}

fn snapshot_error(cause: &'static str, subject: Option<&[u8]>) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidSnapshot, cause, subject)
}
