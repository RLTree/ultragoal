use super::*;

impl AnchoredDirectory {
    pub(crate) fn open_absolute(path: &Path) -> Result<Self, HostFailure> {
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
        Self::from_file(path.to_path_buf(), unsafe { File::from_raw_fd(fd) })
    }

    pub(crate) fn from_file(path: PathBuf, file: File) -> Result<Self, HostFailure> {
        let metadata = file.metadata().map_err(|_| HostFailure::Invalid)?;
        let observed = identity(&metadata);
        // SAFETY: geteuid has no preconditions and only reads the process credential.
        let effective_uid = unsafe { libc::geteuid() };
        if !metadata.is_dir()
            || observed.owner != effective_uid
            || observed.mode & 0o7777 != 0o700
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
        })
    }

    pub(crate) fn open_child(&self, name: &str) -> Result<Self, HostFailure> {
        validate_name(name)?;
        let descriptor = openat(&self.file, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        Self::from_file(self.path.join(name), descriptor)
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

pub(crate) fn openat(
    parent: &File,
    name: &str,
    flags: i32,
    mode: u32,
) -> Result<File, HostFailure> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| HostFailure::Invalid)?;
    // SAFETY: the descriptor is borrowed, the C string is NUL-terminated, and the flags are bounded constants.
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            mode,
        )
    };
    if descriptor < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => HostFailure::Unavailable,
            Some(libc::EEXIST) => HostFailure::Invalid,
            _ => HostFailure::Invalid,
        });
    }
    // SAFETY: a non-negative descriptor was returned exclusively to this call.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}
