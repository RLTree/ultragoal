use super::*;

pub(crate) struct ParentStateLock {
    file: File,
    identity: FileIdentity,
    path: PathBuf,
}

impl ParentStateLock {
    pub(crate) fn acquire_shared(parent: &AnchoredDirectory) -> Result<Self, HostFailure> {
        Self::acquire(parent, libc::LOCK_SH | libc::LOCK_NB)
    }

    fn acquire(parent: &AnchoredDirectory, operation: libc::c_int) -> Result<Self, HostFailure> {
        parent.verify(true)?;
        let file = parent
            .file
            .try_clone()
            .map_err(|_| HostFailure::Persistence)?;
        // SAFETY: this value owns the cloned descriptor and releases only its
        // advisory lock in Drop. Directory locking creates no filesystem row.
        if unsafe { libc::flock(file.as_raw_fd(), operation) } != 0 {
            return Err(HostFailure::Unavailable);
        }
        let value = Self {
            file,
            identity: parent.identity,
            path: parent.path.clone(),
        };
        value.verify(parent)?;
        Ok(value)
    }

    pub(crate) fn verify(&self, parent: &AnchoredDirectory) -> Result<(), HostFailure> {
        if !identity(&self.file.metadata().map_err(|_| HostFailure::Invalid)?)
            .same_directory(self.identity)
            || !parent.identity.same_directory(self.identity)
        {
            return Err(HostFailure::Invalid);
        }
        parent.verify(true)
    }

    pub(crate) fn verify_path(&self) -> Result<(), HostFailure> {
        let opened = AnchoredDirectory::open_identity(&self.path)?;
        if !identity(&self.file.metadata().map_err(|_| HostFailure::Invalid)?)
            .same_directory(self.identity)
            || !opened.same_directory(self.identity)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }
}

impl Drop for ParentStateLock {
    fn drop(&mut self) {
        // SAFETY: this value owns the descriptor and releases only its advisory lock.
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
