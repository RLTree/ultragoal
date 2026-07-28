use super::*;

pub(in crate::context) enum Observation {
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

pub(in crate::context) struct ObservationSet {
    pub(crate) entries: RefCell<BTreeMap<PathBuf, Observation>>,
}

impl ReadSession {
    #[cfg(test)]
    pub(crate) fn observation_count(&self) -> usize {
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

    pub(crate) fn revalidate_directories(&self) -> Result<(), ContextError> {
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

    pub(crate) fn observe_open_regular_identity(
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
