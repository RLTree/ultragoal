use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::owned_compile_directory::{entry_identity, open_directory_at};

pub(crate) struct ForeignCustody {
    entry: File,
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

impl ForeignCustody {
    pub(crate) fn capture(parent: &File, name: &CStr) -> Option<Self> {
        let entry = open_directory_at(parent.as_raw_fd(), name).ok()?;
        let metadata = entry.metadata().ok()?;
        Some(Self {
            entry,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub(crate) fn matches(&self, parent: &File, name: &CStr) -> bool {
        entry_identity(parent.as_raw_fd(), name) == Some((self.device, self.inode))
    }

    pub(crate) fn current_name(&self, parent_path: &Path) -> Option<CString> {
        let mut bytes = [0_i8; libc::PATH_MAX as usize];
        if unsafe { libc::fcntl(self.entry.as_raw_fd(), libc::F_GETPATH, bytes.as_mut_ptr()) } != 0
        {
            return None;
        }
        let path = Path::new(
            std::ffi::CStr::from_bytes_until_nul(unsafe {
                std::slice::from_raw_parts(bytes.as_ptr().cast::<u8>(), bytes.len())
            })
            .ok()?
            .to_str()
            .ok()?,
        );
        if path.parent()? != parent_path {
            return None;
        }
        CString::new(path.file_name()?.as_bytes()).ok()
    }
}
