use super::filesystem::{self, FileIdentity, VerifiedParent};
use super::locking::{LockDeadline, lock_identity};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Keeps a store path bound to one regular file after the first append creates it.
#[derive(Clone, Debug)]
pub(super) struct BoundStoreIdentity {
    value: Arc<Mutex<Option<FileIdentity>>>,
    parent: Arc<VerifiedParent>,
}

impl BoundStoreIdentity {
    pub(super) fn new(value: Option<FileIdentity>, parent: VerifiedParent) -> Self {
        Self {
            value: Arc::new(Mutex::new(value)),
            parent: Arc::new(parent),
        }
    }

    pub(super) fn expected(&self, deadline: &LockDeadline) -> Result<Option<FileIdentity>, String> {
        lock_identity(&self.value, deadline).map(|value| *value)
    }

    /// Serializes only creation/binding, leaving the actual append under the file lock.
    pub(super) fn open_for_append(
        &self,
        deadline: &LockDeadline,
    ) -> Result<(File, FileIdentity), String> {
        let mut guard = lock_identity(&self.value, deadline)?;
        let (file, created) = filesystem::open_append(&self.parent, *guard)?;
        let actual = filesystem::file_identity(&file)?;
        let expected = match *guard {
            Some(expected) => expected,
            None if created => {
                *guard = Some(actual);
                actual
            }
            None => {
                return Err(
                    "observe-store-path-denied: store materialized outside append".to_owned(),
                );
            }
        };
        if expected != actual {
            return Err("observe-store-path-denied: path substitution detected".to_owned());
        }
        filesystem::validate_bound_parent(&self.parent, &file, Some(expected))?;
        Ok((file, expected))
    }

    pub(super) fn validate_bound(
        &self,
        path: &Path,
        file: &File,
        expected: FileIdentity,
    ) -> Result<(), String> {
        let _ = path;
        filesystem::validate_bound_parent(&self.parent, file, Some(expected))
    }

    pub(super) fn parent(&self) -> &VerifiedParent {
        &self.parent
    }

    #[cfg(test)]
    pub(super) fn hold_mutex_for_test(
        &self,
        ready: std::sync::mpsc::Sender<()>,
        release: std::sync::mpsc::Receiver<()>,
    ) -> Result<(), String> {
        let guard = self
            .value
            .try_lock()
            .map_err(|_| "observe-store-test-identity-holder-failed".to_owned())?;
        ready
            .send(())
            .map_err(|_| "observe-store-test-identity-ready-failed".to_owned())?;
        release
            .recv()
            .map_err(|_| "observe-store-test-identity-release-failed".to_owned())?;
        drop(guard);
        Ok(())
    }
}
