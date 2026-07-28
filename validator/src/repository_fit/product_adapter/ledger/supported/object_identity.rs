use super::*;

pub(crate) fn exact_identity(
    store: &Store,
    name: &str,
    file: &File,
    expected_mode: u32,
) -> Result<FileIdentity, LedgerError> {
    let named = store.exact_stat(name)?.ok_or_else(tampered)?;
    let opened = file_identity(&file.metadata().map_err(|_| tampered())?);
    // SAFETY: `geteuid` reads the calling process's effective UID and has no preconditions.
    let expected_uid = unsafe { libc::geteuid() };
    if named != opened
        || opened.device != store.root_identity.device
        || opened.links != 1
        || opened.uid != expected_uid
        || opened.mode & libc::S_IFMT as u32 != libc::S_IFREG as u32
        || opened.mode & 0o7777 != expected_mode
    {
        return Err(tampered());
    }
    Ok(opened)
}

pub(crate) fn read_key(file: &File) -> Result<LedgerKey, LedgerError> {
    let bytes = read_bounded(file, KEY_BYTES as u64)?;
    let bytes: [u8; KEY_BYTES] = bytes.try_into().map_err(|_| tampered())?;
    Ok(LedgerKey(bytes))
}

pub(crate) fn read_bounded(file: &File, maximum: u64) -> Result<Vec<u8>, LedgerError> {
    let metadata = file.metadata().map_err(|_| ledger_io())?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(tampered());
    }
    let mut file = file.try_clone().map_err(|_| ledger_io())?;
    file.seek(SeekFrom::Start(0)).map_err(|_| ledger_io())?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    (&mut file)
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| ledger_io())?;
    let after = file.metadata().map_err(|_| ledger_io())?;
    if bytes.len() as u64 != metadata.len()
        || after.len() != metadata.len()
        || after.mtime() != metadata.mtime()
        || after.mtime_nsec() != metadata.mtime_nsec()
        || after.ctime() != metadata.ctime()
        || after.ctime_nsec() != metadata.ctime_nsec()
    {
        return Err(tampered());
    }
    Ok(bytes)
}

pub(crate) fn descriptor_path(file: &File) -> Result<PathBuf, LedgerError> {
    let request = format!("/dev/fd/{}", file.as_raw_fd());
    let request = CString::new(request).map_err(|_| invalid_store())?;
    let mut buffer = vec![0_i8; libc::PATH_MAX as usize];
    // SAFETY: `file` supplies a live descriptor, and `buffer` is writable for `PATH_MAX` bytes.
    if unsafe {
        libc::fcntl(
            file.as_raw_fd(),
            libc::F_GETPATH,
            buffer.as_mut_ptr().cast::<libc::c_void>(),
        )
    } != 0
    {
        return Err(tampered());
    }
    // SAFETY: successful `F_GETPATH` writes a NUL-terminated path into `buffer`.
    let path = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    if path.to_bytes().first() != Some(&b'/') {
        return Err(tampered());
    }
    let _ = request;
    Ok(PathBuf::from(OsStr::from_bytes(path.to_bytes())))
}

pub(crate) fn root_identity(metadata: &fs::Metadata) -> RootIdentity {
    RootIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        mode: metadata.mode() & 0o7777,
    }
}

pub(crate) fn file_identity(metadata: &fs::Metadata) -> FileIdentity {
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        mode: metadata.mode(),
        length: metadata.len(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

pub(crate) fn stat_identity(stat: &libc::stat) -> FileIdentity {
    FileIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino,
        links: stat.st_nlink as u64,
        uid: stat.st_uid,
        gid: stat.st_gid,
        mode: stat.st_mode as u32,
        length: stat.st_size as u64,
        changed_seconds: stat.st_ctime,
        changed_nanoseconds: stat.st_ctime_nsec,
    }
}

pub(crate) fn temporary_name() -> Result<String, LedgerError> {
    let mut random = [0u8; 16];
    fill(&mut random).map_err(|_| ledger_io())?;
    Ok(format!(
        ".repository-fit-ledger-{}.tmp",
        random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

pub(crate) fn rename_relative(
    directory: &File,
    source: &str,
    target: &str,
) -> Result<(), LedgerError> {
    let source = CString::new(source).map_err(|_| invalid_store())?;
    let target = CString::new(target).map_err(|_| invalid_store())?;
    // SAFETY: `directory` is a live directory descriptor and both names are NUL-terminated.
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

pub(crate) fn unlink_relative(directory: &File, name: &str) -> Result<(), LedgerError> {
    let name = CString::new(name).map_err(|_| invalid_store())?;
    // SAFETY: `directory` is a live directory descriptor and `name` is NUL-terminated.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(ledger_io());
    }
    Ok(())
}

pub(crate) fn decode_digest(value: &str) -> Result<[u8; 32], LedgerError> {
    if !valid_digest(value) {
        return Err(tampered());
    }
    let mut bytes = [0u8; 32];
    for (index, pair) in value.as_bytes()[7..].chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(pair).map_err(|_| tampered())?;
        bytes[index] = u8::from_str_radix(pair, 16).map_err(|_| tampered())?;
    }
    Ok(bytes)
}

pub(crate) fn validate_name(name: &str) -> Result<(), LedgerError> {
    if name.is_empty()
        || name.len() > 255
        || name.contains('/')
        || name == "."
        || name == ".."
        || name.as_bytes().contains(&0)
    {
        return Err(invalid_store());
    }
    Ok(())
}

pub(crate) const fn invalid_store() -> LedgerError {
    LedgerError::new(LedgerErrorId::InvalidStore)
}

pub(crate) const fn tampered() -> LedgerError {
    LedgerError::new(LedgerErrorId::Tampered)
}

pub(crate) const fn replay_error() -> LedgerError {
    LedgerError::new(LedgerErrorId::Replay)
}

pub(crate) const fn active_lease() -> LedgerError {
    LedgerError::new(LedgerErrorId::ActiveLease)
}
