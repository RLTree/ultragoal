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
}
