use super::{AnchoredDirectory, HostFailure};
use std::fs::File;
use std::os::fd::AsRawFd;

/// Process-scoped coordination for the complete `routine-public` owner.
///
/// The stable parent directory descriptor is itself the lock object. This
/// deliberately creates no lock file, so read-only diagnosis remains
/// zero-write even for legacy stores that predate this boundary.
pub(crate) struct ParentStateLock {
    _file: File,
}

impl Drop for ParentStateLock {
    fn drop(&mut self) {
        // SAFETY: `_file` remains live for this entire call. Explicit unlock is
        // required because it is a duplicate of the anchored parent
        // descriptor; closing only the duplicate would leave the shared open
        // file description locked until the anchor itself closed.
        let _ = unsafe { libc::flock(self._file.as_raw_fd(), libc::LOCK_UN) };
    }
}

impl ParentStateLock {
    pub(crate) fn shared(parent: &AnchoredDirectory) -> Result<Self, HostFailure> {
        Self::acquire(parent, libc::LOCK_SH)
    }

    pub(crate) fn exclusive(parent: &AnchoredDirectory) -> Result<Self, HostFailure> {
        Self::acquire(parent, libc::LOCK_EX)
    }

    fn acquire(parent: &AnchoredDirectory, operation: libc::c_int) -> Result<Self, HostFailure> {
        parent.verify()?;
        let file = parent.file.try_clone().map_err(|_| HostFailure::Invalid)?;
        // SAFETY: `file` owns a live descriptor for the stable parent. The
        // non-blocking advisory lock is released when the owned descriptor is
        // dropped and never outlives it.
        if unsafe { libc::flock(file.as_raw_fd(), operation | libc::LOCK_NB) } != 0 {
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(value) if value == libc::EWOULDBLOCK || value == libc::EAGAIN => {
                    HostFailure::Busy
                }
                _ => HostFailure::Invalid,
            });
        }
        parent.verify()?;
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::successor_public::routine::host::supported::DirectorySecurity;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn private_directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hul-parent-state-lock-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[test]
    fn shared_readers_exclude_an_exclusive_migration_writer() {
        let path = private_directory();
        let first =
            AnchoredDirectory::open_absolute(&path, DirectorySecurity::PrivateAuthority).unwrap();
        let second =
            AnchoredDirectory::open_absolute(&path, DirectorySecurity::PrivateAuthority).unwrap();
        let shared = ParentStateLock::shared(&first).unwrap();
        assert!(matches!(
            ParentStateLock::exclusive(&second),
            Err(HostFailure::Busy)
        ));
        drop(shared);
        ParentStateLock::exclusive(&second).unwrap();
        drop((second, first));
        std::fs::remove_dir(path).unwrap();
    }

    #[test]
    fn exclusive_writer_excludes_new_readers() {
        let path = private_directory();
        let first =
            AnchoredDirectory::open_absolute(&path, DirectorySecurity::PrivateAuthority).unwrap();
        let second =
            AnchoredDirectory::open_absolute(&path, DirectorySecurity::PrivateAuthority).unwrap();
        let exclusive = ParentStateLock::exclusive(&first).unwrap();
        assert!(matches!(
            ParentStateLock::shared(&second),
            Err(HostFailure::Busy)
        ));
        drop(exclusive);
        ParentStateLock::shared(&second).unwrap();
        drop((second, first));
        std::fs::remove_dir(path).unwrap();
    }
}
