fn require_lock_identity(
    store: &Store,
    lock: &ProcessLock,
    expected: FileIdentity,
) -> Result<(), HostEffectLedgerError> {
    let observed = exact_identity(store, LOCK_NAME, lock.file(), 0)?;
    validate_lock_identity(observed)?;
    if observed != expected {
        return Err(tampered());
    }
    Ok(())
}

fn exact_identity(
    store: &Store,
    name: &str,
    file: &File,
    max_bytes: u64,
) -> Result<FileIdentity, HostEffectLedgerError> {
    let descriptor = FileIdentity::from_metadata(&file.metadata().map_err(|_| ledger_io())?);
    let named = store.exact_stat(name)?.ok_or_else(tampered)?;
    validate_regular(descriptor, max_bytes, 0o600)?;
    if descriptor != named {
        return Err(tampered());
    }
    Ok(descriptor)
}

fn open_relative(
    directory: &File,
    name: &str,
    flags: i32,
    mode: u32,
) -> Result<File, HostEffectLedgerError> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| invalid_record())?;
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags, mode) };
    if descriptor < 0 {
        return Err(ledger_io());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn stat_relative(
    directory: &File,
    name: &str,
) -> Result<Option<FileIdentity>, HostEffectLedgerError> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| invalid_record())?;
    let mut value = std::mem::MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            value.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ENOENT) {
            return Ok(None);
        }
        return Err(ledger_io());
    }
    let value = unsafe { value.assume_init() };
    if value.st_size < 0 {
        return Err(tampered());
    }
    Ok(Some(FileIdentity {
        device: value.st_dev as u64,
        inode: value.st_ino as u64,
        mode: value.st_mode as u32,
        hard_links: value.st_nlink as u64,
        length: value.st_size as u64,
    }))
}

fn rename_relative(
    directory: &File,
    source: &str,
    target: &str,
) -> Result<(), HostEffectLedgerError> {
    validate_name(source)?;
    validate_name(target)?;
    let source = CString::new(source).map_err(|_| invalid_record())?;
    let target = CString::new(target).map_err(|_| invalid_record())?;
    if unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            source.as_ptr(),
            directory.as_raw_fd(),
            target.as_ptr(),
        )
    } != 0
    {
        return Err(ledger_io());
    }
    Ok(())
}

fn unlink_relative(directory: &File, name: &str) -> Result<(), HostEffectLedgerError> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| invalid_record())?;
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(ledger_io());
    }
    Ok(())
}

fn validate_name(value: &str) -> Result<(), HostEffectLedgerError> {
    if value.is_empty()
        || value.len() > 160
        || value.as_bytes().contains(&0)
        || value.contains('/')
        || value == "."
        || value == ".."
    {
        return Err(invalid_record());
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), HostEffectLedgerError> {
    if !valid_id(value) {
        return Err(invalid_record());
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn invalid_record() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord)
}

fn invalid_transition() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidTransition)
}

fn stale_head() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::StaleHead)
}

fn replay_error() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Replay)
}

fn tampered() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Tampered)
}

fn ledger_io() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Io)
}
