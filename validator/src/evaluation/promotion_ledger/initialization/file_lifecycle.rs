/// Safely opens a promotion-ledger file during its durable creation lifecycle.
fn open_promotion_initialization_file(
    root: &File,
    name: &str,
    create: bool,
    append: bool,
) -> Result<File, PromotionLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    if append {
        flags |= libc::O_APPEND;
    }
    let file = unsafe { File::from_raw_fd(openat(root, name, flags, 0o600).map_err(map_storage)?) };
    let identity = safe_file_identity(&file).map_err(map_storage)?;
    if identity.mode & libc::S_IFMT as u32 != libc::S_IFREG as u32
        || identity.mode & 0o777 != 0o600
        || !(1..=2).contains(&identity.links)
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-file-unsafe",
        ));
    }
    Ok(file)
}

fn recover_or_create_promotion_initialization_file(
    root: &File,
    name: &str,
    append: bool,
) -> Result<File, PromotionLedgerError> {
    if entry_exists(root, name).map_err(map_storage)? {
        let file = open_promotion_initialization_file(root, name, false, append)?;
        if safe_file_identity(&file).map_err(map_storage)?.length != 0 {
            return Ok(file);
        }
        unlink_promotion_initialization_file(root, name)?;
    }
    open_promotion_initialization_file(root, name, true, append)
}

fn write_promotion_initialization_json(
    file: &File,
    value: &impl Serialize,
) -> Result<(), PromotionLedgerError> {
    if safe_file_identity(file).map_err(map_storage)?.length != 0 {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-file-not-empty",
        ));
    }
    let bytes = serde_json::to_vec(value)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-serialization-failed"))?;
    let mut writer = file;
    writer
        .write_all(&bytes)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-write-failed"))?;
    file.sync_all()
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-fsync-failed"))
}

fn read_promotion_initialization_json<T: for<'de> Deserialize<'de>>(
    file: &File,
) -> Result<T, PromotionLedgerError> {
    let identity = safe_file_identity(file).map_err(map_storage)?;
    if identity.length == 0 || identity.length > MAX_LEDGER_BYTES {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-file-unsafe",
        ));
    }
    let mut bytes = Vec::with_capacity(identity.length as usize);
    let reader = file;
    reader
        .take(MAX_LEDGER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-read-failed"))?;
    if bytes.len() as u64 != identity.length
        || safe_file_identity(file).map_err(map_storage)? != identity
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-file-changed",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-malformed"))
}

fn cleanup_same_initialization_file(
    root: &File,
    temporary: &str,
    named: &File,
    append: bool,
) -> Result<(), PromotionLedgerError> {
    if !entry_exists(root, temporary).map_err(map_storage)? {
        return Ok(());
    }
    let scratch = open_promotion_initialization_file(root, temporary, false, append)?;
    if safe_file_identity(&scratch)
        .map_err(map_storage)?
        .authority()
        != safe_file_identity(named).map_err(map_storage)?.authority()
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-ambiguous",
        ));
    }
    unlink_promotion_initialization_file(root, temporary)?;
    sync_directory(root).map_err(map_storage)
}

fn publish_promotion_initialization_file(
    root: &File,
    temporary: &str,
    destination: &str,
    expected: FileAuthorityIdentity,
) -> Result<(), PromotionLedgerError> {
    let temporary = std::ffi::CString::new(temporary)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-component-invalid"))?;
    let destination_name = std::ffi::CString::new(destination)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-component-invalid"))?;
    if unsafe {
        libc::linkat(
            root.as_raw_fd(),
            temporary.as_ptr(),
            root.as_raw_fd(),
            destination_name.as_ptr(),
            0,
        )
    } != 0
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-publication-conflict",
        ));
    }
    let published = safe_file_identity(&open_promotion_initialization_file(
        root,
        destination,
        false,
        false,
    )?)
    .map_err(map_storage)?
    .authority();
    if published.device != expected.device
        || published.inode != expected.inode
        || published.mode != expected.mode
        || published.links != expected.links + 1
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-publication-substituted",
        ));
    }
    unlink_promotion_initialization_file(
        root,
        temporary.to_str().expect("trusted initialization name"),
    )?;
    sync_directory(root).map_err(map_storage)
}

fn unlink_promotion_initialization_file(
    root: &File,
    name: &str,
) -> Result<(), PromotionLedgerError> {
    let name = std::ffi::CString::new(name)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-component-invalid"))?;
    if unsafe { libc::unlinkat(root.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-cleanup-failed",
        ));
    }
    Ok(())
}
