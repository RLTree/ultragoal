impl Store {
    fn open(root: &Path) -> Result<Self, HostEffectLedgerError> {
        let named = fs::symlink_metadata(root).map_err(|_| ledger_io())?;
        let named_identity = FileIdentity::from_metadata(&named);
        validate_directory(named_identity)?;
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let directory = options.open(root).map_err(|_| ledger_io())?;
        let descriptor = directory.metadata().map_err(|_| ledger_io())?;
        let directory_identity = FileIdentity::from_metadata(&descriptor);
        validate_directory(directory_identity)?;
        if named_identity != directory_identity {
            return Err(tampered());
        }
        let store = Self {
            root: root.to_path_buf(),
            canonical_root: fs::canonicalize(root).map_err(|_| ledger_io())?,
            directory: Arc::new(directory),
            directory_identity,
        };
        store.verify_root()?;
        Ok(store)
    }

    fn verify_root(&self) -> Result<(), HostEffectLedgerError> {
        let named = fs::symlink_metadata(&self.root).map_err(|_| tampered())?;
        let descriptor = self.directory.metadata().map_err(|_| ledger_io())?;
        let named_identity = FileIdentity::from_metadata(&named);
        let descriptor_identity = FileIdentity::from_metadata(&descriptor);
        validate_directory(named_identity)?;
        validate_directory(descriptor_identity)?;
        if !named_identity.same_directory_anchor(self.directory_identity)
            || !descriptor_identity.same_directory_anchor(self.directory_identity)
            || fs::canonicalize(&self.root).map_err(|_| tampered())? != self.canonical_root
        {
            return Err(tampered());
        }
        Ok(())
    }

    fn validate_complete(
        &self,
        expected_lock_identity: FileIdentity,
    ) -> Result<(), HostEffectLedgerError> {
        self.verify_root()?;
        let mut observed = BTreeSet::new();
        for entry in fs::read_dir(&self.root).map_err(|_| ledger_io())? {
            let entry = entry.map_err(|_| ledger_io())?;
            let name = entry.file_name().into_string().map_err(|_| tampered())?;
            if !observed.insert(name.clone()) {
                return Err(tampered());
            }
            let identity = self.exact_stat(&name)?.ok_or_else(tampered)?;
            match name.as_str() {
                LOCK_NAME => {
                    validate_lock_identity(identity)?;
                    if identity != expected_lock_identity {
                        return Err(tampered());
                    }
                }
                KEY_NAME => {
                    validate_regular(identity, KEY_BYTES as u64, 0o600)?;
                    if identity.length != KEY_BYTES as u64 {
                        return Err(tampered());
                    }
                }
                STATE_NAME => {
                    validate_regular(identity, MAX_LEDGER_BYTES, 0o600)?;
                    if identity.length == 0 {
                        return Err(tampered());
                    }
                }
                _ => return Err(tampered()),
            }
        }
        let expected = BTreeSet::from([
            KEY_NAME.to_owned(),
            LOCK_NAME.to_owned(),
            STATE_NAME.to_owned(),
        ]);
        self.verify_root()?;
        if observed != expected {
            return Err(tampered());
        }
        Ok(())
    }

    fn open_or_create_lock(&self) -> Result<File, HostEffectLedgerError> {
        self.verify_root()?;
        let file = open_relative(
            &self.directory,
            LOCK_NAME,
            libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )?;
        exact_identity(self, LOCK_NAME, &file, 0)?;
        self.directory.sync_all().map_err(|_| ledger_io())?;
        Ok(file)
    }

    fn open_existing(&self, name: &str, max_bytes: u64) -> Result<File, HostEffectLedgerError> {
        self.verify_root()?;
        let access = if name == LOCK_NAME {
            libc::O_RDWR
        } else {
            libc::O_RDONLY
        };
        let file = open_relative(
            &self.directory,
            name,
            access | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        exact_identity(self, name, &file, max_bytes)?;
        Ok(file)
    }

    fn open_or_create_key(&self) -> Result<(LedgerKey, FileIdentity), HostEffectLedgerError> {
        self.verify_root()?;
        if self.exact_stat(KEY_NAME)?.is_none() {
            let mut bytes = [0_u8; KEY_BYTES];
            fill(&mut bytes).map_err(|_| ledger_io())?;
            let created = open_relative(
                &self.directory,
                KEY_NAME,
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            );
            match created {
                Ok(mut file) => {
                    file.write_all(&bytes).map_err(|_| ledger_io())?;
                    file.sync_all().map_err(|_| ledger_io())?;
                    self.directory.sync_all().map_err(|_| ledger_io())?;
                }
                Err(_error) if self.exact_stat(KEY_NAME)?.is_some() => {}
                Err(error) => return Err(error),
            }
            for byte in &mut bytes {
                unsafe { std::ptr::write_volatile(byte, 0) };
            }
        }
        self.open_existing_key(|| Ok(()))
    }

    fn open_existing_key(
        &self,
        after_identity: impl FnOnce() -> Result<(), HostEffectLedgerError>,
    ) -> Result<(LedgerKey, FileIdentity), HostEffectLedgerError> {
        let mut file = self.open_existing(KEY_NAME, KEY_BYTES as u64)?;
        let identity = exact_identity(self, KEY_NAME, &file, KEY_BYTES as u64)?;
        if identity.length != KEY_BYTES as u64 {
            return Err(tampered());
        }
        after_identity()?;
        let bytes = self.read_bound_file(KEY_NAME, &mut file, identity, KEY_BYTES as u64, 0o600)?;
        let key: [u8; KEY_BYTES] = bytes.try_into().map_err(|_| tampered())?;
        Ok((LedgerKey(key), identity))
    }

    fn read_exact_file(
        &self,
        name: &str,
        max_bytes: u64,
        required_mode: u32,
    ) -> Result<Vec<u8>, HostEffectLedgerError> {
        self.verify_root()?;
        let mut file = self.open_existing(name, max_bytes)?;
        let identity = exact_identity(self, name, &file, max_bytes)?;
        self.read_bound_file(name, &mut file, identity, max_bytes, required_mode)
    }
}
