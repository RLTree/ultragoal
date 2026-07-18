use super::*;

impl Store {
    pub(crate) fn open(root: &Path) -> Result<Self, LedgerError> {
        let supplied = root.to_path_buf();
        let path_metadata = fs::symlink_metadata(&supplied).map_err(|_| invalid_store())?;
        if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
            return Err(invalid_store());
        }
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let directory = options.open(&supplied).map_err(|_| invalid_store())?;
        let metadata = directory.metadata().map_err(|_| invalid_store())?;
        let expected_uid = unsafe { libc::geteuid() };
        if !metadata.is_dir()
            || metadata.dev() != path_metadata.dev()
            || metadata.ino() != path_metadata.ino()
            || metadata.uid() != expected_uid
            || path_metadata.uid() != expected_uid
            || metadata.mode() & 0o7777 != 0o700
            || path_metadata.mode() & 0o7777 != 0o700
        {
            return Err(invalid_store());
        }
        let canonical_root = fs::canonicalize(&supplied).map_err(|_| invalid_store())?;
        if !canonical_root.is_absolute() {
            return Err(invalid_store());
        }
        let value = Self {
            requested_root: supplied,
            canonical_root,
            directory: Arc::new(directory),
            root_identity: root_identity(&metadata),
        };
        value.verify_root()?;
        Ok(value)
    }
    pub(crate) fn verify_root(&self) -> Result<(), LedgerError> {
        let path = fs::symlink_metadata(&self.requested_root).map_err(|_| tampered())?;
        let opened = self.directory.metadata().map_err(|_| tampered())?;
        if path.file_type().is_symlink()
            || !path.is_dir()
            || root_identity(&path) != self.root_identity
            || root_identity(&opened) != self.root_identity
            || path.uid() != unsafe { libc::geteuid() }
            || path.mode() & 0o7777 != 0o700
            || fs::canonicalize(&self.requested_root).map_err(|_| tampered())?
                != self.canonical_root
            || descriptor_path(&self.directory)? != self.canonical_root
        {
            return Err(tampered());
        }
        Ok(())
    }
    pub(crate) fn names(&self) -> Result<BTreeSet<String>, LedgerError> {
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
            return Err(ledger_io());
        }
        let stream = unsafe { libc::fdopendir(descriptor) };
        if stream.is_null() {
            unsafe { libc::close(descriptor) };
            return Err(ledger_io());
        }
        let mut names = BTreeSet::new();
        let result = loop {
            unsafe { *libc::__error() = 0 };
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break if unsafe { *libc::__error() } == 0 {
                    Ok(names)
                } else {
                    Err(ledger_io())
                };
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            if names.len() >= MAX_STORE_ENTRIES {
                break Err(tampered());
            }
            let name = std::str::from_utf8(bytes).map_err(|_| tampered())?;
            if !matches!(name, KEY_NAME | LOCK_NAME | STATE_NAME) {
                break Err(tampered());
            }
            if !names.insert(name.to_owned()) {
                break Err(tampered());
            }
        };
        if unsafe { libc::closedir(stream) } != 0 {
            return Err(ledger_io());
        }
        self.verify_root()?;
        result
    }
    pub(crate) fn is_empty_unclassified(&self) -> Result<bool, LedgerError> {
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
            return Err(ledger_io());
        }
        let stream = unsafe { libc::fdopendir(descriptor) };
        if stream.is_null() {
            unsafe { libc::close(descriptor) };
            return Err(ledger_io());
        }
        let result = loop {
            unsafe { *libc::__error() = 0 };
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break if unsafe { *libc::__error() } == 0 {
                    Ok(true)
                } else {
                    Err(ledger_io())
                };
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if !matches!(bytes, b"." | b"..") {
                break Ok(false);
            }
        };
        if unsafe { libc::closedir(stream) } != 0 {
            return Err(ledger_io());
        }
        self.verify_root()?;
        result
    }
    pub(crate) fn open_or_create_lock(&self) -> Result<File, LedgerError> {
        match self.create_exclusive(LOCK_NAME, 0o600) {
            Ok(file) => {
                file.sync_all().map_err(|_| ledger_io())?;
                self.directory.sync_all().map_err(|_| ledger_io())?;
                Ok(file)
            }
            Err(error) if error.id() == LedgerErrorId::Replay => {
                self.open_existing(LOCK_NAME, libc::O_RDWR)
            }
            Err(error) => Err(error),
        }
    }
    pub(crate) fn create_key(&self) -> Result<File, LedgerError> {
        let mut bytes = [0u8; KEY_BYTES];
        fill(&mut bytes).map_err(|_| ledger_io())?;
        let mut file = self.create_exclusive(KEY_NAME, 0o600)?;
        file.write_all(&bytes).map_err(|_| ledger_io())?;
        file.sync_all().map_err(|_| ledger_io())?;
        self.directory.sync_all().map_err(|_| ledger_io())?;
        Ok(file)
    }
}
