use super::*;

impl Store {
    pub(crate) fn stat_name(&self, name: &str) -> Result<Option<FileIdentity>, RoutineError> {
        validate_name(name)?;
        let name =
            CString::new(name).map_err(|_| error("routine-production-authority-name-invalid"))?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ENOENT) => Ok(None),
                _ => Err(error("routine-production-authority-entry-stat-failed")),
            };
        }
        Ok(Some(stat_identity(&unsafe { stat.assume_init() })))
    }
    pub(crate) fn names(&self) -> Result<BTreeSet<String>, RoutineError> {
        self.verify_root()?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_CLOEXEC
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(error("routine-production-authority-root-read-failed"));
        }
        let stream = unsafe { libc::fdopendir(descriptor) };
        if stream.is_null() {
            unsafe { libc::close(descriptor) };
            return Err(error("routine-production-authority-root-read-failed"));
        }
        let mut names = BTreeSet::new();
        let result = loop {
            unsafe { *libc::__error() = 0 };
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break if unsafe { *libc::__error() } == 0 {
                    Ok(names)
                } else {
                    Err(error("routine-production-authority-root-read-failed"))
                };
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            let name = std::str::from_utf8(bytes)
                .map_err(|_| error("routine-production-authority-entry-name-invalid"))?;
            if !matches!(name, KEY_NAME | LOCK_NAME | STATE_NAME) || !names.insert(name.to_owned())
            {
                break Err(error("routine-production-authority-entry-unknown"));
            }
        };
        if unsafe { libc::closedir(stream) } != 0 {
            return Err(error("routine-production-authority-root-read-failed"));
        }
        result
    }
    pub(crate) fn read_state(&self) -> Result<Vec<u8>, RoutineError> {
        let file = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
        let _ = self.exact_identity(STATE_NAME, &file, 0o600)?;
        read_bounded(&file, MAX_STATE_BYTES)
    }
    pub(crate) fn state_identity(&self) -> Result<FileIdentity, RoutineError> {
        let file = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
        self.exact_identity(STATE_NAME, &file, 0o600)
    }
    pub(crate) fn validate_complete(
        &self,
        key_identity: FileIdentity,
        lock_identity: FileIdentity,
    ) -> Result<(), RoutineError> {
        if self.names()?
            != BTreeSet::from([
                KEY_NAME.to_owned(),
                LOCK_NAME.to_owned(),
                STATE_NAME.to_owned(),
            ])
        {
            return Err(error("routine-production-authority-store-incomplete"));
        }
        let key = self.open_existing(KEY_NAME, libc::O_RDONLY)?;
        let lock = self.open_existing(LOCK_NAME, libc::O_RDONLY)?;
        let state = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
        if self.exact_identity(KEY_NAME, &key, 0o600)? != key_identity
            || self.exact_identity(LOCK_NAME, &lock, 0o600)? != lock_identity
            || read_bounded(&lock, LOCK_MARKER.len() as u64)? != LOCK_MARKER
        {
            return Err(error("routine-production-authority-store-mutated"));
        }
        let _ = self.exact_identity(STATE_NAME, &state, 0o600)?;
        self.verify_root()
    }
}

impl ProcessLock {
    pub(crate) fn acquire(file: File) -> Result<Self, RoutineError> {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(error("routine-production-authority-lock-busy"));
        }
        Ok(Self(file))
    }
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}
