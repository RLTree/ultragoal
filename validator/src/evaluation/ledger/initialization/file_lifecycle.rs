/// Safely opens a ledger file during its durable creation lifecycle.
fn open_initialization_file(
    root: &File,
    name: &str,
    create: bool,
    append: bool,
) -> Result<File, EvaluationLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if append {
        flags |= libc::O_APPEND;
    }
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, name, flags, 0o600)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32
        || identity.mode & 0o777 != 0o600
        || !(1..=2).contains(&identity.links)
    {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-file-unsafe",
        ));
    }
    Ok(file)
}

fn write_initialization_json(
    file: &File,
    value: &impl Serialize,
) -> Result<(), EvaluationLedgerError> {
    if safe_file_identity(file)?.length != 0 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-file-not-empty",
        ));
    }
    let bytes = serde_json::to_vec(value)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    let mut writer = file;
    writer
        .write_all(&bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-write-failed"))?;
    file.sync_all()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-fsync-failed"))
}

fn read_initialization_json<T: for<'de> Deserialize<'de>>(
    file: &File,
) -> Result<T, EvaluationLedgerError> {
    let identity = safe_file_identity(file)?;
    if identity.length == 0 || identity.length > MAX_LEDGER_BYTES {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-file-unsafe",
        ));
    }
    let mut bytes = Vec::with_capacity(identity.length as usize);
    let reader = file;
    reader
        .take(MAX_LEDGER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-read-failed"))?;
    if bytes.len() as u64 != identity.length || safe_file_identity(file)? != identity {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-file-changed",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-malformed"))
}

fn publish_initialization_file(
    root: &File,
    temporary: &str,
    destination: &str,
    expected: FileAuthorityIdentity,
) -> Result<(), EvaluationLedgerError> {
    let source = c_string(temporary)?;
    let destination_name = c_string(destination)?;
    if unsafe {
        libc::linkat(
            root.as_raw_fd(),
            source.as_ptr(),
            root.as_raw_fd(),
            destination_name.as_ptr(),
            0,
        )
    } != 0
    {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-publication-conflict",
        ));
    }
    let published =
        safe_file_identity(&open_initialization_file(root, destination, false, false)?)?
            .authority();
    if published.device != expected.device
        || published.inode != expected.inode
        || published.mode != expected.mode
        || published.links != expected.links + 1
    {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-publication-substituted",
        ));
    }
    unlink_initialization_file(root, temporary)?;
    sync_directory(root)
}

fn unlink_initialization_file(root: &File, name: &str) -> Result<(), EvaluationLedgerError> {
    let name = c_string(name)?;
    if unsafe { libc::unlinkat(root.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-cleanup-failed",
        ));
    }
    Ok(())
}
