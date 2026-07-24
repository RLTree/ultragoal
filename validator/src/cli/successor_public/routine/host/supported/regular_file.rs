use super::super::{AnchoredDirectory, HostFailure, identity, validate_name};
use super::file_descriptor::openat;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::fd::AsRawFd;

impl AnchoredDirectory {
    pub(crate) fn remove_regular(&self, name: &str) -> Result<(), HostFailure> {
        validate_name(name)?;
        let file = self.open_regular(name, libc::O_RDONLY, 0o600)?;
        let expected = identity(&file.metadata().map_err(|_| HostFailure::Invalid)?);
        if self.stat(name)? != Some(expected) {
            return Err(HostFailure::Busy);
        }
        let name = CString::new(name).map_err(|_| HostFailure::Invalid)?;
        // SAFETY: `name` is validated and the descriptor anchors the parent.
        if unsafe { libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(HostFailure::Busy);
        }
        self.file.sync_all().map_err(|_| HostFailure::Invalid)
    }

    pub(crate) fn open_regular(
        &self,
        name: &str,
        flags: i32,
        mode: u32,
    ) -> Result<File, HostFailure> {
        let file = openat(&self.file, name, flags, 0)?;
        let metadata = file.metadata().map_err(|_| HostFailure::Invalid)?;
        let observed = identity(&metadata);
        // SAFETY: geteuid has no preconditions and only reads the process credential.
        let effective_uid = unsafe { libc::geteuid() };
        if !metadata.is_file()
            || observed.owner != effective_uid
            || observed.mode & 0o7777 != mode
            || observed.links != 1
            || self.stat(name)? != Some(observed)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(file)
    }

    pub(crate) fn open_or_create_regular(
        &self,
        name: &str,
        mode: u32,
    ) -> Result<(File, bool), HostFailure> {
        let created = openat(
            &self.file,
            name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            mode,
        );
        match created {
            Ok(file) => {
                let metadata = file.metadata().map_err(|_| HostFailure::Invalid)?;
                let observed = identity(&metadata);
                // SAFETY: geteuid has no preconditions and only reads the process credential.
                let effective_uid = unsafe { libc::geteuid() };
                if !metadata.is_file()
                    || observed.owner != effective_uid
                    || observed.mode & 0o7777 != mode
                    || observed.links != 1
                    || self.stat(name)? != Some(observed)
                {
                    return Err(HostFailure::Invalid);
                }
                self.file.sync_all().map_err(|_| HostFailure::Invalid)?;
                Ok((file, true))
            }
            Err(HostFailure::Invalid) => Ok((self.open_regular(name, libc::O_RDWR, mode)?, false)),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn replace_regular_atomically(
        &self,
        name: &str,
        stage_name: &str,
        mode: u32,
        bytes: &[u8],
        expected_existing: bool,
    ) -> Result<(), HostFailure> {
        validate_name(name)?;
        validate_name(stage_name)?;
        let expected_identity = if expected_existing {
            let current = self.open_regular(name, libc::O_RDONLY, mode)?;
            Some(identity(
                &current.metadata().map_err(|_| HostFailure::Invalid)?,
            ))
        } else {
            self.stat(name)?
        };
        if expected_identity.is_some() != expected_existing || self.stat(stage_name)?.is_some() {
            return Err(HostFailure::Busy);
        }
        let (mut stage, created) = self.open_or_create_regular(stage_name, mode)?;
        if !created {
            return Err(HostFailure::Busy);
        }
        stage.write_all(bytes).map_err(|_| HostFailure::Invalid)?;
        stage.sync_all().map_err(|_| HostFailure::Invalid)?;
        let stage_identity = identity(&stage.metadata().map_err(|_| HostFailure::Invalid)?);
        if self.stat(name)? != expected_identity || self.stat(stage_name)? != Some(stage_identity) {
            return Err(HostFailure::Busy);
        }
        let source = CString::new(stage_name).map_err(|_| HostFailure::Invalid)?;
        let destination = CString::new(name).map_err(|_| HostFailure::Invalid)?;
        let flags = if expected_existing {
            0
        } else {
            libc::RENAME_EXCL
        };
        // SAFETY: both names are validated, both descriptors are the same live
        // parent, and the caller holds the host process lock for the replacement.
        let result = unsafe {
            libc::renameatx_np(
                self.file.as_raw_fd(),
                source.as_ptr(),
                self.file.as_raw_fd(),
                destination.as_ptr(),
                flags,
            )
        };
        if result != 0 {
            return Err(HostFailure::Busy);
        }
        self.file.sync_all().map_err(|_| HostFailure::Invalid)?;
        if self.stat(name)? != Some(stage_identity) {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }
}
