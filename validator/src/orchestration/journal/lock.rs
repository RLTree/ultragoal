use super::super::OrchestrationError;
use super::store::Store;
use std::fs::File;

#[cfg(unix)]
use std::os::fd::AsRawFd;

pub(crate) struct JournalLock {
    file: File,
}

impl JournalLock {
    pub(crate) fn acquire(store: &Store) -> Result<Self, OrchestrationError> {
        acquire(store)
    }
}

#[cfg(unix)]
fn acquire(store: &Store) -> Result<JournalLock, OrchestrationError> {
    let file = store.open_lock()?;
    // SAFETY: `file` remains open for the entire lock lifetime and its file
    // descriptor is valid for `flock`.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(OrchestrationError::JournalIo);
    }
    store.verify_root()?;
    store.validate_lock_entry()?;
    store.sync_root()?;
    Ok(JournalLock { file })
}

#[cfg(not(unix))]
fn acquire(_: &Store) -> Result<JournalLock, OrchestrationError> {
    Err(OrchestrationError::JournalIo)
}

impl Drop for JournalLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        // SAFETY: `self.file` still owns the descriptor acquired by `flock`;
        // unlock is best-effort during drop and cannot outlive that descriptor.
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
