use super::*;

impl AnchoredDirectory {
    pub(crate) fn open_absolute(path: &Path, exact_owner_only: bool) -> Result<Self, HostFailure> {
        let encoded =
            CString::new(path.as_os_str().as_bytes()).map_err(|_| HostFailure::Invalid)?;
        // SAFETY: the owned C string is NUL-terminated and all flags are constants.
        let descriptor = unsafe {
            libc::open(
                encoded.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(HostFailure::Unavailable);
        }
        // SAFETY: a non-negative descriptor was returned exclusively to this call.
        let file = unsafe { File::from_raw_fd(descriptor) };
        Self::finish(path.to_path_buf(), file, exact_owner_only)
    }

    pub(crate) fn open_child(
        &self,
        name: &str,
        exact_owner_only: bool,
    ) -> Result<Self, HostFailure> {
        self.verify(false)?;
        let file = openat(
            &self.file,
            name,
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
            0,
        )?;
        Self::finish(self.path.join(name), file, exact_owner_only)
    }

    pub(crate) fn finish(
        path: PathBuf,
        file: File,
        exact_owner_only: bool,
    ) -> Result<Self, HostFailure> {
        let metadata = file.metadata().map_err(|_| HostFailure::Invalid)?;
        let identity = identity(&metadata);
        // SAFETY: geteuid has no preconditions and only reads the process credential.
        let effective_uid = unsafe { libc::geteuid() };
        if !metadata.is_dir()
            || identity.uid != effective_uid
            || identity.mode & 0o022 != 0
            || (exact_owner_only && identity.mode != 0o700)
            || fs::canonicalize(&path).map_err(|_| HostFailure::Invalid)? != path
        {
            return Err(HostFailure::Invalid);
        }
        let value = Self {
            path,
            file,
            identity,
        };
        value.verify(exact_owner_only)?;
        Ok(value)
    }

    pub(crate) fn verify(&self, exact_owner_only: bool) -> Result<(), HostFailure> {
        if !directory_matches(&self.path, &self.file, self.identity, exact_owner_only) {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }

    pub(crate) fn open_identity(path: &Path) -> Result<FileIdentity, HostFailure> {
        let encoded =
            CString::new(path.as_os_str().as_bytes()).map_err(|_| HostFailure::Invalid)?;
        // SAFETY: the owned C string is NUL-terminated and all flags are constants.
        let descriptor = unsafe {
            libc::open(
                encoded.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(HostFailure::Invalid);
        }
        // SAFETY: a non-negative descriptor was returned exclusively to this call.
        let file = unsafe { File::from_raw_fd(descriptor) };
        file.metadata()
            .map(|metadata| identity(&metadata))
            .map_err(|_| HostFailure::Invalid)
    }
}

pub(crate) fn directory_matches(
    path: &Path,
    file: &File,
    expected: FileIdentity,
    exact_owner_only: bool,
) -> bool {
    let Ok(metadata) = file.metadata() else {
        return false;
    };
    let Ok(reopened) = AnchoredDirectory::open_identity(path) else {
        return false;
    };
    let observed = identity(&metadata);
    // SAFETY: geteuid has no preconditions and only reads the process credential.
    let effective_uid = unsafe { libc::geteuid() };
    observed.same_directory(expected)
        && reopened.same_directory(expected)
        && metadata.is_dir()
        && observed.uid == effective_uid
        && observed.mode & 0o022 == 0
        && (!exact_owner_only || observed.mode == 0o700)
        && fs::canonicalize(path)
            .map(|canonical| canonical == path)
            .unwrap_or(false)
}

pub(crate) struct ProcessLock {
    pub(crate) file: File,
    pub(crate) identity: FileIdentity,
}
