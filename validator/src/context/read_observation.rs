use super::error::{ContextError, io_error};
use super::read_revalidation;
use super::read_session::ReadSession;
use super::read_snapshot::{FileSnapshot, snapshot};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

enum Observation {
    Regular {
        digest: Option<String>,
        byte_length: u64,
        identity: FileSnapshot,
        descriptor: File,
    },
    Symlink {
        digest: String,
        identity: FileSnapshot,
    },
}

pub(super) struct ObservationSet {
    entries: RefCell<BTreeMap<PathBuf, Observation>>,
}

impl ReadSession {
    #[cfg(test)]
    pub(super) fn observation_count(&self) -> usize {
        self.observations.entries.borrow().len()
    }

    pub(crate) fn observe_presence_parent(&self, path: &Path) -> Result<(), ContextError> {
        if path == self.root() {
            self.pin_directory(path)?;
            return Ok(());
        }
        let target = self.target(path)?;
        let mut ancestor = target.parent();
        while let Some(path) = ancestor {
            if !path.starts_with(self.root()) {
                break;
            }
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                    self.pin_directory(path)?;
                    return Ok(());
                }
                Ok(_) => ancestor = path.parent(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    ancestor = path.parent();
                }
                Err(error) => return Err(io_error(path, error)),
            }
        }
        Err(ContextError::PathDenied(
            "no confined parent exists for presence observation".to_owned(),
        ))
    }

    pub(super) fn revalidate_directories(&self) -> Result<(), ContextError> {
        let directories = self.directories.borrow();
        let identities = self.directory_identities.borrow();
        for (path, directory) in directories.iter() {
            let expected = identities.get(path).ok_or_else(|| {
                ContextError::ConcurrentMutation("pinned directory identity is missing".to_owned())
            })?;
            let descriptor = directory
                .metadata()
                .map_err(|error| io_error(path, error))?;
            let current = fs::metadata(path).map_err(|error| io_error(path, error))?;
            if snapshot(&descriptor) != *expected || snapshot(&current) != *expected {
                return Err(ContextError::ConcurrentMutation(format!(
                    "pinned directory changed: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }

    pub(crate) fn observe_regular(
        &self,
        path: &Path,
        digest: &str,
        byte_length: u64,
        metadata: &fs::Metadata,
        file: &File,
    ) -> Result<(), ContextError> {
        self.observations
            .regular(self.target(path)?, digest, byte_length, metadata, file)
    }

    pub(super) fn observe_open_regular_identity(
        &self,
        path: &Path,
        metadata: &fs::Metadata,
        file: &File,
    ) -> Result<(), ContextError> {
        self.observations
            .regular_identity(self.target(path)?, metadata.len(), metadata, file)
    }

    pub(crate) fn observe_symlink(
        &self,
        path: &Path,
        digest: &str,
        metadata: &fs::Metadata,
    ) -> Result<(), ContextError> {
        self.observations
            .symlink(self.target(path)?, digest, metadata)
    }
}

impl ObservationSet {
    pub(super) fn new() -> Self {
        Self {
            entries: RefCell::new(BTreeMap::new()),
        }
    }

    pub(super) fn regular(
        &self,
        path: PathBuf,
        digest: &str,
        byte_length: u64,
        metadata: &fs::Metadata,
        file: &File,
    ) -> Result<(), ContextError> {
        self.record_regular(path, Some(digest), byte_length, metadata, file)
    }

    pub(super) fn regular_identity(
        &self,
        path: PathBuf,
        byte_length: u64,
        metadata: &fs::Metadata,
        file: &File,
    ) -> Result<(), ContextError> {
        self.record_regular(path, None, byte_length, metadata, file)
    }

    fn record_regular(
        &self,
        path: PathBuf,
        digest: Option<&str>,
        byte_length: u64,
        metadata: &fs::Metadata,
        file: &File,
    ) -> Result<(), ContextError> {
        let identity = snapshot(metadata);
        let mut entries = self.entries.borrow_mut();
        if let Some(existing) = entries.get_mut(&path) {
            return match existing {
                Observation::Regular {
                    digest: known,
                    byte_length: known_length,
                    identity: known_identity,
                    ..
                } if *known_length == byte_length && known_identity == &identity => {
                    match (known.as_deref(), digest) {
                        (Some(left), Some(right)) if left != right => Err(concurrent(&path)),
                        (None, Some(value)) => {
                            *known = Some(value.to_owned());
                            Ok(())
                        }
                        _ => Ok(()),
                    }
                }
                _ => Err(concurrent(&path)),
            };
        }
        let descriptor = file.try_clone().map_err(|error| io_error(&path, error))?;
        entries.insert(
            path,
            Observation::Regular {
                digest: digest.map(ToOwned::to_owned),
                byte_length,
                identity,
                descriptor,
            },
        );
        Ok(())
    }

    pub(super) fn symlink(
        &self,
        path: PathBuf,
        digest: &str,
        metadata: &fs::Metadata,
    ) -> Result<(), ContextError> {
        let identity = snapshot(metadata);
        if let Some(existing) = self.entries.borrow().get(&path) {
            return match existing {
                Observation::Symlink {
                    digest: known,
                    identity: known_identity,
                } if known == digest && known_identity == &identity => Ok(()),
                _ => Err(concurrent(&path)),
            };
        }
        self.entries.borrow_mut().insert(
            path,
            Observation::Symlink {
                digest: digest.to_owned(),
                identity,
            },
        );
        Ok(())
    }

    pub(super) fn revalidate(&self) -> Result<(), ContextError> {
        let entries = self.entries.borrow();
        let mut total = 0_u64;
        for (path, observation) in entries.iter() {
            match observation {
                Observation::Regular {
                    digest,
                    byte_length,
                    identity,
                    descriptor,
                } => {
                    if digest.is_some() {
                        total = total.saturating_add(*byte_length);
                        if total > super::read_session::MAX_READ_SESSION_BYTES {
                            return Err(ContextError::PathDenied(
                                "observed-file revalidation exceeds the session byte limit"
                                    .to_owned(),
                            ));
                        }
                    }
                    read_revalidation::regular(
                        path,
                        digest.as_deref(),
                        *byte_length,
                        identity,
                        descriptor,
                    )?;
                }
                Observation::Symlink { digest, identity } => {
                    read_revalidation::symlink(path, digest, identity)?;
                }
            }
        }
        Ok(())
    }
}

fn concurrent(path: &Path) -> ContextError {
    ContextError::ConcurrentMutation(format!("read-session observation: {}", path.display()))
}
