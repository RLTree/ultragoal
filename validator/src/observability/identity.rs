use super::filesystem::{self, FileIdentity, VerifiedParent};
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

    pub(super) fn expected(&self) -> Result<Option<FileIdentity>, String> {
        self.value
            .lock()
            .map(|value| *value)
            .map_err(|_| "observe-store-lock-failed".to_owned())
    }

    /// Serializes only creation/binding, leaving the actual append under the file lock.
    pub(super) fn open_for_append(&self) -> Result<File, String> {
        let mut guard = self
            .value
            .lock()
            .map_err(|_| "observe-store-lock-failed".to_owned())?;
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
        Ok(file)
    }

    pub(super) fn validate_bound(&self, path: &Path, file: &File) -> Result<(), String> {
        let _ = path;
        let expected = self.expected()?.ok_or_else(|| {
            "observe-store-path-denied: store materialized outside append".to_owned()
        })?;
        filesystem::validate_bound_parent(&self.parent, file, Some(expected))
    }

    pub(super) fn parent(&self) -> &VerifiedParent {
        &self.parent
    }
}
