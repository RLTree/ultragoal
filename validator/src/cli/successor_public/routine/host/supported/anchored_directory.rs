use super::*;

impl AnchoredDirectory {
    pub(crate) fn open_absolute(path: &Path) -> Result<Self, HostFailure> {
        let name = CString::new(path.as_os_str().as_bytes()).map_err(|_| HostFailure::Invalid)?;
        let fd = unsafe {
            libc::open(
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if fd < 0 {
            return Err(HostFailure::Unavailable);
        }
        Self::from_file(path.to_path_buf(), unsafe { File::from_raw_fd(fd) })
    }

    pub(crate) fn from_file(path: PathBuf, file: File) -> Result<Self, HostFailure> {
        let metadata = file.metadata().map_err(|_| HostFailure::Invalid)?;
        let observed = identity(&metadata);
        if !metadata.is_dir()
            || observed.owner != unsafe { libc::geteuid() }
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
        if !metadata.is_file()
            || observed.owner != unsafe { libc::geteuid() }
            || observed.mode & 0o7777 != mode
            || observed.links != 1
            || self.stat(name)? != Some(observed)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(file)
    }

    pub(crate) fn open_optional_regular(
        &self,
        name: &str,
        mode: u32,
    ) -> Result<Option<File>, HostFailure> {
        match self.stat(name)? {
            None => Ok(None),
            Some(_) => self.open_regular(name, libc::O_RDONLY, mode).map(Some),
        }
    }

    pub(crate) fn create_exclusive(&self, name: &str, mode: u32) -> Result<File, HostFailure> {
        let file = openat(
            &self.file,
            name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            mode,
        )?;
        let metadata = file.metadata().map_err(|_| HostFailure::Persistence)?;
        let observed = identity(&metadata);
        if observed.owner != unsafe { libc::geteuid() }
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
        let result = unsafe {
            libc::fstatat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result == 0 {
            let metadata = unsafe { metadata.assume_init() };
            return Ok(Some(stat_identity(&metadata)));
        }
        if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            Ok(None)
        } else {
            Err(HostFailure::Invalid)
        }
    }

    pub(crate) fn rename(&self, from: &str, to: &str) -> Result<(), HostFailure> {
        validate_name(from)?;
        validate_name(to)?;
        let from = CString::new(from).map_err(|_| HostFailure::Persistence)?;
        let to = CString::new(to).map_err(|_| HostFailure::Persistence)?;
        if unsafe {
            libc::renameat(
                self.file.as_raw_fd(),
                from.as_ptr(),
                self.file.as_raw_fd(),
                to.as_ptr(),
            )
        } != 0
        {
            return Err(HostFailure::Persistence);
        }
        Ok(())
    }

    pub(crate) fn unlink(&self, name: &str) -> Result<(), HostFailure> {
        validate_name(name)?;
        let name = CString::new(name).map_err(|_| HostFailure::Persistence)?;
        if unsafe { libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(HostFailure::Persistence);
        }
        Ok(())
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
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(HostFailure::Invalid);
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
    Ok(unsafe { File::from_raw_fd(descriptor) })
}
