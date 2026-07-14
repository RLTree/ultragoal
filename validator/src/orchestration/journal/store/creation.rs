impl Store {
    pub(crate) fn create(root: &Path) -> Result<Self, OrchestrationError> {
        match fs::symlink_metadata(root) {
            Ok(metadata) => validate_directory(&metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                builder.mode(0o700);
                builder
                    .create(root)
                    .map_err(|_| OrchestrationError::JournalIo)?;
            }
            Err(_) => return Err(OrchestrationError::JournalIo),
        }
        Self::open(root)
    }

    pub(crate) fn open(root: &Path) -> Result<Self, OrchestrationError> {
        let path_metadata =
            fs::symlink_metadata(root).map_err(|_| OrchestrationError::JournalCorrupt)?;
        validate_directory(&path_metadata)?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        options.custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let directory = options
            .open(root)
            .map_err(|_| OrchestrationError::JournalCorrupt)?;
        let descriptor_metadata = directory
            .metadata()
            .map_err(|_| OrchestrationError::JournalIo)?;
        validate_directory(&descriptor_metadata)?;
        #[cfg(unix)]
        if path_metadata.dev() != descriptor_metadata.dev()
            || path_metadata.ino() != descriptor_metadata.ino()
        {
            return Err(OrchestrationError::JournalCorrupt);
        }
        let value = Self {
            root: root.to_path_buf(),
            canonical_root: fs::canonicalize(root).map_err(|_| OrchestrationError::JournalIo)?,
            directory: Arc::new(directory),
            #[cfg(unix)]
            device: descriptor_metadata.dev(),
            #[cfg(unix)]
            inode: descriptor_metadata.ino(),
        };
        value.verify_root()?;
        Ok(value)
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn validate_journal_entries(&self) -> Result<(), OrchestrationError> {
        for name in ["events.jsonl", "head.json", "journal.lock"] {
            if !self.exact_entry(name)? {
                return Err(OrchestrationError::JournalCorrupt);
            }
        }
        Ok(())
    }

    pub(crate) fn validate_lock_entry(&self) -> Result<(), OrchestrationError> {
        if self.exact_entry("journal.lock")? {
            Ok(())
        } else {
            Err(OrchestrationError::JournalCorrupt)
        }
    }

    pub(crate) fn require_absent(&self, name: &str) -> Result<(), OrchestrationError> {
        if self.exact_entry(name)? {
            Err(OrchestrationError::JournalConflict)
        } else {
            Ok(())
        }
    }

    pub(crate) fn verify_root(&self) -> Result<(), OrchestrationError> {
        let path_metadata =
            fs::symlink_metadata(&self.root).map_err(|_| OrchestrationError::JournalCorrupt)?;
        validate_directory(&path_metadata)?;
        let descriptor_metadata = self
            .directory
            .metadata()
            .map_err(|_| OrchestrationError::JournalIo)?;
        validate_directory(&descriptor_metadata)?;
        if fs::canonicalize(&self.root).map_err(|_| OrchestrationError::JournalCorrupt)?
            != self.canonical_root
        {
            return Err(OrchestrationError::JournalCorrupt);
        }
        #[cfg(unix)]
        if path_metadata.dev() != self.device
            || path_metadata.ino() != self.inode
            || descriptor_metadata.dev() != self.device
            || descriptor_metadata.ino() != self.inode
        {
            return Err(OrchestrationError::JournalCorrupt);
        }
        Ok(())
    }

    pub(crate) fn read(&self, name: &str, limit: u64) -> Result<Vec<u8>, OrchestrationError> {
        self.read_with(name, limit, || {})
    }

    #[cfg(test)]
    pub(crate) fn read_log_with_hook(
        &self,
        hook: impl FnOnce(),
    ) -> Result<Vec<u8>, OrchestrationError> {
        self.read_with("events.jsonl", MAX_JOURNAL_BYTES, hook)
    }

    fn read_with(
        &self,
        name: &str,
        limit: u64,
        hook: impl FnOnce(),
    ) -> Result<Vec<u8>, OrchestrationError> {
        self.verify_root()?;
        let named_before = self
            .exact_stat(name)?
            .ok_or(OrchestrationError::JournalCorrupt)?;
        hook();
        let mut file = super::sys::open_relative(
            &self.directory,
            name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        let before = file.metadata().map_err(|_| OrchestrationError::JournalIo)?;
        validate_regular(&before, limit)?;
        let named_open = self
            .exact_stat(name)?
            .ok_or(OrchestrationError::JournalCorrupt)?;
        if named_before != named_open || !same_named_file(&before, &named_open) {
            return Err(OrchestrationError::JournalCorrupt);
        }
        let mut bytes = Vec::with_capacity(before.len().min(limit) as usize);
        Read::by_ref(&mut file)
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| OrchestrationError::JournalIo)?;
        let after = file.metadata().map_err(|_| OrchestrationError::JournalIo)?;
        validate_regular(&after, limit)?;
        let named_after = self
            .exact_stat(name)?
            .ok_or(OrchestrationError::JournalCorrupt)?;
        if bytes.len() as u64 > limit
            || !same_file(&before, &after)
            || named_after != named_open
            || !same_named_file(&after, &named_after)
            || after.len() != bytes.len() as u64
        {
            return Err(OrchestrationError::JournalCorrupt);
        }
        self.verify_root()?;
        Ok(bytes)
    }

    pub(crate) fn read_log(&self) -> Result<Vec<u8>, OrchestrationError> {
        self.read("events.jsonl", MAX_JOURNAL_BYTES)
    }

    pub(crate) fn open_lock(&self) -> Result<File, OrchestrationError> {
        self.verify_root()?;
        let existing = self.exact_stat("journal.lock")?;
        let create = if existing.is_some() {
            0
        } else {
            libc::O_CREAT | libc::O_EXCL
        };
        let file = super::sys::open_relative(
            &self.directory,
            "journal.lock",
            libc::O_RDWR | create | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )?;
        let metadata = file.metadata().map_err(|_| OrchestrationError::JournalIo)?;
        validate_regular(&metadata, 128)?;
        let named = self
            .exact_stat("journal.lock")?
            .ok_or(OrchestrationError::JournalCorrupt)?;
        if existing.is_some_and(|prior| prior != named) || !same_named_file(&metadata, &named) {
            return Err(OrchestrationError::JournalCorrupt);
        }
        Ok(file)
    }
}
