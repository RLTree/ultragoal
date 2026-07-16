use super::*;

impl Store {
    pub(super) fn open(root: &Path) -> Result<Self, RoutineError> {
        let supplied = root.to_path_buf();
        let path_metadata = fs::symlink_metadata(&supplied)
            .map_err(|_| error("routine-production-authority-root-missing"))?;
        if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
            return Err(error("routine-production-authority-root-invalid"));
        }
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let directory = options
            .open(&supplied)
            .map_err(|_| error("routine-production-authority-root-open-failed"))?;
        let opened = directory
            .metadata()
            .map_err(|_| error("routine-production-authority-root-stat-failed"))?;
        let expected_uid = unsafe { libc::geteuid() };
        if !opened.is_dir()
            || opened.dev() != path_metadata.dev()
            || opened.ino() != path_metadata.ino()
            || opened.uid() != expected_uid
            || path_metadata.uid() != expected_uid
            || opened.mode() & 0o7777 != 0o700
            || path_metadata.mode() & 0o7777 != 0o700
        {
            return Err(error(
                "routine-production-authority-root-permissions-invalid",
            ));
        }
        let canonical_root = fs::canonicalize(&supplied)
            .map_err(|_| error("routine-production-authority-root-canonicalize-failed"))?;
        if !canonical_root.is_absolute() {
            return Err(error("routine-production-authority-root-not-absolute"));
        }
        let store = Self {
            requested_root: supplied,
            canonical_root,
            directory: Arc::new(directory),
            identity: root_identity(&opened),
        };
        store.verify_root()?;
        Ok(store)
    }
    pub(super) fn verify_root(&self) -> Result<(), RoutineError> {
        let path = fs::symlink_metadata(&self.requested_root)
            .map_err(|_| error("routine-production-authority-root-replaced"))?;
        let opened = self
            .directory
            .metadata()
            .map_err(|_| error("routine-production-authority-root-replaced"))?;
        if path.file_type().is_symlink()
            || !path.is_dir()
            || root_identity(&path) != self.identity
            || root_identity(&opened) != self.identity
            || path.uid() != unsafe { libc::geteuid() }
            || path.mode() & 0o7777 != 0o700
            || fs::canonicalize(&self.requested_root)
                .map_err(|_| error("routine-production-authority-root-replaced"))?
                != self.canonical_root
            || descriptor_path(&self.directory)? != self.canonical_root
        {
            return Err(error("routine-production-authority-root-replaced"));
        }
        Ok(())
    }
    pub(super) fn acquire_initial_lock(&self) -> Result<ProcessLock, RoutineError> {
        self.verify_root()?;
        let file = self.create_exclusive(LOCK_NAME, 0o600)?;
        let mut guard = ProcessLock::acquire(file)?;
        guard
            .0
            .write_all(LOCK_MARKER)
            .map_err(|_| error("routine-production-authority-lock-write-failed"))?;
        guard
            .0
            .sync_all()
            .map_err(|_| error("routine-production-authority-lock-sync-failed"))?;
        self.directory
            .sync_all()
            .map_err(|_| error("routine-production-authority-root-sync-failed"))?;
        if read_bounded(&guard.0, LOCK_MARKER.len() as u64)? != LOCK_MARKER {
            return Err(error("routine-production-authority-lock-invalid"));
        }
        Ok(guard)
    }
    pub(super) fn create_key(&self) -> Result<File, RoutineError> {
        let mut bytes = [0u8; KEY_BYTES];
        getrandom::fill(&mut bytes)
            .map_err(|_| error("routine-production-authority-random-unavailable"))?;
        let mut file = self.create_exclusive(KEY_NAME, 0o600)?;
        file.write_all(&bytes)
            .map_err(|_| error("routine-production-authority-key-write-failed"))?;
        file.sync_all()
            .map_err(|_| error("routine-production-authority-key-sync-failed"))?;
        self.directory
            .sync_all()
            .map_err(|_| error("routine-production-authority-root-sync-failed"))?;
        Ok(file)
    }
    pub(super) fn create_exclusive(&self, name: &str, mode: u32) -> Result<File, RoutineError> {
        validate_name(name)?;
        let name =
            CString::new(name).map_err(|_| error("routine-production-authority-name-invalid"))?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_EXCL
                    | libc::O_CLOEXEC
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK,
                mode,
            )
        };
        if descriptor < 0 {
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EEXIST) => error("routine-production-authority-entry-exists"),
                _ => error("routine-production-authority-entry-create-failed"),
            });
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
    pub(super) fn open_existing(&self, name: &str, flags: i32) -> Result<File, RoutineError> {
        validate_name(name)?;
        let name =
            CString::new(name).map_err(|_| error("routine-production-authority-name-invalid"))?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(error("routine-production-authority-entry-open-failed"));
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
    pub(super) fn exact_identity(
        &self,
        name: &str,
        file: &File,
        mode: u32,
    ) -> Result<FileIdentity, RoutineError> {
        let opened = file
            .metadata()
            .map_err(|_| error("routine-production-authority-entry-stat-failed"))?;
        let path = self
            .stat_name(name)?
            .ok_or_else(|| error("routine-production-authority-entry-missing"))?;
        let opened = file_identity(&opened);
        if opened != path
            || opened.owner != unsafe { libc::geteuid() }
            || opened.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG)
            || opened.mode & 0o7777 != mode
            || opened.links != 1
        {
            return Err(error("routine-production-authority-entry-identity-invalid"));
        }
        Ok(opened)
    }
}
