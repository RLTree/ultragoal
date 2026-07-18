use super::model::Observation;
use super::*;

impl ObservationSet {
    pub(crate) fn new() -> Self {
        Self {
            entries: RefCell::new(BTreeMap::new()),
        }
    }

    pub(crate) fn regular(
        &self,
        path: PathBuf,
        digest: &str,
        byte_length: u64,
        metadata: &fs::Metadata,
        file: &File,
    ) -> Result<(), ContextError> {
        self.record_regular(path, Some(digest), byte_length, metadata, file)
    }

    pub(crate) fn regular_identity(
        &self,
        path: PathBuf,
        byte_length: u64,
        metadata: &fs::Metadata,
        file: &File,
    ) -> Result<(), ContextError> {
        self.record_regular(path, None, byte_length, metadata, file)
    }

    pub(crate) fn record_regular(
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

    pub(crate) fn symlink(
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

    pub(crate) fn revalidate(&self) -> Result<(), ContextError> {
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
                        if total > super::super::read_session::MAX_READ_SESSION_BYTES {
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

pub(crate) fn concurrent(path: &Path) -> ContextError {
    ContextError::ConcurrentMutation(format!("read-session observation: {}", path.display()))
}
