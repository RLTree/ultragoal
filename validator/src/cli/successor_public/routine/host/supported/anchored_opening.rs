use super::super::{AnchoredDirectory, DirectorySecurity, HostFailure, identity, validate_name};
use super::file_descriptor::openat;
use std::ffi::CString;
use std::fs::{self, File};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

impl AnchoredDirectory {
    pub(crate) fn open_absolute(
        path: &Path,
        security: DirectorySecurity,
    ) -> Result<Self, HostFailure> {
        let name = CString::new(path.as_os_str().as_bytes()).map_err(|_| HostFailure::Invalid)?;
        // SAFETY: the owned C string is NUL-terminated and all flags are constants.
        let fd = unsafe {
            libc::open(
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if fd < 0 {
            return Err(HostFailure::Unavailable);
        }
        // SAFETY: a non-negative descriptor was returned exclusively to this call.
        Self::from_file(
            path.to_path_buf(),
            unsafe { File::from_raw_fd(fd) },
            security,
        )
    }

    pub(crate) fn from_file(
        path: PathBuf,
        file: File,
        security: DirectorySecurity,
    ) -> Result<Self, HostFailure> {
        let metadata = file.metadata().map_err(|_| HostFailure::Invalid)?;
        let observed = identity(&metadata);
        // SAFETY: geteuid has no preconditions and only reads the process credential.
        let effective_uid = unsafe { libc::geteuid() };
        if !metadata.is_dir()
            || observed.owner != effective_uid
            || !security.accepts(observed.mode)
            || fs::symlink_metadata(&path)
                .map(|value| identity(&value) != observed)
                .unwrap_or(true)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(Self {
            path,
            file,
            identity: observed,
            security,
        })
    }

    pub(crate) fn open_child(&self, name: &str) -> Result<Self, HostFailure> {
        self.open_child_with_security(name, self.security)
    }

    pub(crate) fn open_child_with_security(
        &self,
        name: &str,
        security: DirectorySecurity,
    ) -> Result<Self, HostFailure> {
        validate_name(name)?;
        let descriptor = openat(&self.file, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        Self::from_file(self.path.join(name), descriptor, security)
    }

    pub(crate) fn duplicate(&self) -> Result<Self, HostFailure> {
        let file = self.file.try_clone().map_err(|_| HostFailure::Invalid)?;
        Self::from_file(self.path.clone(), file, self.security)
    }

    pub(crate) fn open_or_create_owned_child(
        &self,
        name: &str,
    ) -> Result<(Self, bool), HostFailure> {
        self.open_or_create_child_with_security(name, self.security)
    }

    pub(crate) fn open_or_create_child_with_security(
        &self,
        name: &str,
        security: DirectorySecurity,
    ) -> Result<(Self, bool), HostFailure> {
        validate_name(name)?;
        let name = CString::new(name).map_err(|_| HostFailure::Invalid)?;
        // SAFETY: the descriptor is live, the component is validated, and mkdirat retains neither input.
        let created = unsafe { libc::mkdirat(self.file.as_raw_fd(), name.as_ptr(), 0o700) } == 0;
        if !created && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
            return Err(HostFailure::Invalid);
        }
        let child = self
            .open_child_with_security(name.to_str().map_err(|_| HostFailure::Invalid)?, security)?;
        if created {
            self.file.sync_all().map_err(|_| HostFailure::Invalid)?;
        }
        Ok((child, created))
    }
}
