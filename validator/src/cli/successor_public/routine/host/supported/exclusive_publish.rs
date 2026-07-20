use super::super::{AnchoredDirectory, HostFailure, validate_name};
use std::ffi::CString;
use std::os::fd::AsRawFd;

impl AnchoredDirectory {
    pub(crate) fn publish_child_exclusive(
        &self,
        source: &str,
        destination: &str,
    ) -> Result<(), HostFailure> {
        validate_name(source)?;
        validate_name(destination)?;
        let source = CString::new(source).map_err(|_| HostFailure::Invalid)?;
        let destination = CString::new(destination).map_err(|_| HostFailure::Invalid)?;
        // SAFETY: both names are validated, both descriptors are the same live parent, and RENAME_EXCL prevents replacement.
        let result = unsafe {
            libc::renameatx_np(
                self.file.as_raw_fd(),
                source.as_ptr(),
                self.file.as_raw_fd(),
                destination.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        if result != 0 {
            return Err(HostFailure::Invalid);
        }
        self.file.sync_all().map_err(|_| HostFailure::Invalid)
    }
}
