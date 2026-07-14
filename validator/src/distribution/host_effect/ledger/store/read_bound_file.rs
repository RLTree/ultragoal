impl Store {
    fn read_bound_file(
        &self,
        name: &str,
        file: &mut File,
        expected_identity: FileIdentity,
        max_bytes: u64,
        required_mode: u32,
    ) -> Result<Vec<u8>, HostEffectLedgerError> {
        self.verify_root()?;
        let descriptor_before =
            FileIdentity::from_metadata(&file.metadata().map_err(|_| ledger_io())?);
        let named_before = self.exact_stat(name)?.ok_or_else(tampered)?;
        validate_regular(descriptor_before, max_bytes, required_mode)?;
        if descriptor_before != expected_identity || named_before != expected_identity {
            return Err(tampered());
        }
        let mut bytes = Vec::with_capacity(expected_identity.length.min(max_bytes) as usize);
        Read::by_ref(file)
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| ledger_io())?;
        let descriptor_after =
            FileIdentity::from_metadata(&file.metadata().map_err(|_| ledger_io())?);
        let named_after = self.exact_stat(name)?.ok_or_else(tampered)?;
        if bytes.len() as u64 > max_bytes
            || bytes.len() as u64 != descriptor_after.length
            || descriptor_after != expected_identity
            || descriptor_after != named_after
        {
            return Err(tampered());
        }
        self.verify_root()?;
        Ok(bytes)
    }

    fn write_atomic(
        &self,
        name: &str,
        bytes: &[u8],
        lock: &ProcessLock,
        expected_lock_identity: FileIdentity,
    ) -> Result<(), HostEffectLedgerError> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_LEDGER_BYTES {
            return Err(invalid_record());
        }
        self.verify_root()?;
        let target_before = self.exact_stat(name)?;
        if target_before
            .is_some_and(|identity| validate_regular(identity, MAX_LEDGER_BYTES, 0o600).is_err())
        {
            return Err(tampered());
        }
        let (temp_name, mut temp) = self.create_temp(name)?;
        let result = (|| {
            temp.write_all(bytes).map_err(|_| ledger_io())?;
            temp.sync_all().map_err(|_| ledger_io())?;
            let temp_identity = exact_identity(self, &temp_name, &temp, bytes.len() as u64)?;
            validate_regular(temp_identity, bytes.len() as u64, 0o600)?;
            if temp_identity.length != bytes.len() as u64 {
                return Err(tampered());
            }
            self.verify_root()?;
            if self.exact_stat(name)? != target_before {
                return Err(stale_head());
            }
            require_lock_identity(self, lock, expected_lock_identity)?;
            rename_relative(&self.directory, &temp_name, name)?;
            self.directory.sync_all().map_err(|_| ledger_io())?;
            require_lock_identity(self, lock, expected_lock_identity)?;
            self.verify_root()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = unlink_relative(&self.directory, &temp_name);
        }
        result
    }

    fn create_temp(&self, target: &str) -> Result<(String, File), HostEffectLedgerError> {
        for _ in 0..64 {
            let mut nonce = [0_u8; 8];
            fill(&mut nonce).map_err(|_| ledger_io())?;
            let name = format!(".{target}.{:x}.tmp", u64::from_ne_bytes(nonce));
            match open_relative(
                &self.directory,
                &name,
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            ) {
                Ok(file) => return Ok((name, file)),
                Err(_) if self.exact_stat(&name)?.is_some() => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ledger_io())
    }

    fn exact_stat(&self, name: &str) -> Result<Option<FileIdentity>, HostEffectLedgerError> {
        self.verify_root_shallow()?;
        stat_relative(&self.directory, name)
    }

    fn verify_root_shallow(&self) -> Result<(), HostEffectLedgerError> {
        let descriptor = self.directory.metadata().map_err(|_| ledger_io())?;
        if !FileIdentity::from_metadata(&descriptor).same_directory_anchor(self.directory_identity)
        {
            return Err(tampered());
        }
        Ok(())
    }
}

struct ProcessLock {
    file: File,
}

impl ProcessLock {
    fn acquire(file: File) -> Result<Self, HostEffectLedgerError> {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(ledger_io());
        }
        Ok(Self { file })
    }

    fn file(&self) -> &File {
        &self.file
    }
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

fn create_root(root: &Path) -> Result<(), HostEffectLedgerError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => validate_directory(FileIdentity::from_metadata(&metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(root)
                .map_err(|_| ledger_io())?;
            let metadata = fs::symlink_metadata(root).map_err(|_| ledger_io())?;
            validate_directory(FileIdentity::from_metadata(&metadata))
        }
        Err(_) => Err(ledger_io()),
    }
}

fn validate_directory(identity: FileIdentity) -> Result<(), HostEffectLedgerError> {
    if !identity.directory() || identity.hard_links == 0 || identity.permissions() != 0o700 {
        return Err(tampered());
    }
    Ok(())
}

fn validate_regular(
    identity: FileIdentity,
    max_bytes: u64,
    required_mode: u32,
) -> Result<(), HostEffectLedgerError> {
    if !identity.regular()
        || identity.hard_links != 1
        || identity.length > max_bytes
        || identity.permissions() != required_mode
    {
        return Err(tampered());
    }
    Ok(())
}

fn validate_lock_identity(identity: FileIdentity) -> Result<(), HostEffectLedgerError> {
    validate_regular(identity, 0, 0o600)?;
    if identity.length != 0 {
        return Err(tampered());
    }
    Ok(())
}

fn require_payload_lock(
    payload: &SnapshotPayload,
    expected: FileIdentity,
) -> Result<(), HostEffectLedgerError> {
    validate_lock_identity(payload.lock_identity)?;
    if payload.lock_identity != expected {
        return Err(tampered());
    }
    Ok(())
}
