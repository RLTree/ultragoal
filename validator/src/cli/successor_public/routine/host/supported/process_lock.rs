use super::super::{HostFailure, LOCK_MARKER, ProcessLock};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;

impl ProcessLock {
    pub(crate) fn acquire(file: File) -> Result<Self, HostFailure> {
        // SAFETY: the descriptor is owned by `file` and the lock operation does not outlive it.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(value) if value == libc::EWOULDBLOCK || value == libc::EAGAIN => {
                    HostFailure::Busy
                }
                _ => HostFailure::Invalid,
            });
        }
        Ok(Self(file))
    }
}

pub(crate) fn write_lock_marker(lock: &File) -> Result<(), HostFailure> {
    let mut marker = lock.try_clone().map_err(|_| HostFailure::Invalid)?;
    marker.set_len(0).map_err(|_| HostFailure::Invalid)?;
    marker
        .seek(SeekFrom::Start(0))
        .map_err(|_| HostFailure::Invalid)?;
    marker
        .write_all(LOCK_MARKER)
        .map_err(|_| HostFailure::Invalid)?;
    marker.sync_all().map_err(|_| HostFailure::Invalid)
}
