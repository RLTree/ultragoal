use super::super::{
    AnchoredDirectory, HostFailure, Identity, identity, same_anchored_directory, stat_identity,
    validate_name,
};
use std::ffi::CString;
use std::fs;
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;

impl AnchoredDirectory {
    pub(crate) fn stat(&self, name: &str) -> Result<Option<Identity>, HostFailure> {
        validate_name(name)?;
        let name = CString::new(name).map_err(|_| HostFailure::Invalid)?;
        let mut metadata = MaybeUninit::<libc::stat>::uninit();
        // SAFETY: the descriptor is borrowed, the C string is NUL-terminated, and `metadata` points to writable storage.
        let result = unsafe {
            libc::fstatat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result == 0 {
            // SAFETY: fstatat returned zero, so it initialized `metadata`.
            let metadata = unsafe { metadata.assume_init() };
            return Ok(Some(stat_identity(&metadata)));
        }
        if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            Ok(None)
        } else {
            Err(HostFailure::Invalid)
        }
    }

    pub(crate) fn verify(&self) -> Result<(), HostFailure> {
        let opened = identity(&self.file.metadata().map_err(|_| HostFailure::Invalid)?);
        let path = fs::symlink_metadata(&self.path).map_err(|_| HostFailure::Invalid)?;
        if !same_anchored_directory(opened, self.identity)
            || !same_anchored_directory(identity(&path), self.identity)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }
}
