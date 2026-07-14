pub(super) fn read_directory_names(root: &File) -> Result<Vec<OsString>, EvaluationLedgerError> {
    let before = safe_file_identity(root)?;
    let descriptor = openat(
        root,
        ".",
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        0,
    )?;
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe {
            libc::close(descriptor);
        }
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-directory-read-failed",
        ));
    }
    let stream = DirectoryStream(stream);
    let mut names = Vec::new();
    loop {
        let errno = unsafe { directory_errno_location() };
        unsafe {
            *errno = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if unsafe { *errno } != 0 {
                return Err(EvaluationLedgerError::new(
                    "evaluation-ledger-directory-read-failed",
                ));
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        names.push(OsString::from_vec(name.to_vec()));
    }
    drop(stream);
    if safe_file_identity(root)? != before {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-directory-changed-during-read",
        ));
    }
    names.sort();
    Ok(names)
}

fn open_lock(root: &File, create: bool) -> Result<File, EvaluationLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, LOCK_NAME, flags, 0o600)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32 || identity.links != 1 {
        return Err(EvaluationLedgerError::new("evaluation-ledger-lock-unsafe"));
    }
    Ok(file)
}

fn open_anchor(root: &File, create: bool) -> Result<File, EvaluationLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, ANCHOR_NAME, flags, 0o600)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32 || identity.links != 1 {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-authority-unsafe",
        ));
    }
    Ok(file)
}

fn require_named_lock_identity(
    root: &File,
    expected: FileIdentity,
) -> Result<(), EvaluationLedgerError> {
    let current = open_lock(root, false)?;
    if safe_file_identity(&current)? != expected {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-lock-descriptor-substituted",
        ));
    }
    Ok(())
}

fn require_named_anchor_authority(
    root: &File,
    expected: FileAuthorityIdentity,
) -> Result<(), EvaluationLedgerError> {
    let current = open_anchor(root, false)?;
    if safe_file_identity(&current)?.authority() != expected {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-descriptor-substituted",
        ));
    }
    Ok(())
}

pub(super) fn safe_file_identity(file: &File) -> Result<FileIdentity, EvaluationLedgerError> {
    let metadata = file
        .metadata()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-stat-failed"))?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        length: metadata.len(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
    })
}

fn ensure_only_known_entries(
    path: &Path,
    root: &File,
    initializing: bool,
) -> Result<Option<PendingPublication>, EvaluationLedgerError> {
    test_directory_scan_pause(path);
    let names = read_directory_names(root)?;
    let allowed = [ANCHOR_NAME, LOCK_NAME, STATE_NAME];
    let mut pending = None;
    for name in names {
        if !initializing && allowed.iter().any(|allowed| name == OsStr::new(allowed)) {
            continue;
        }
        let Some(name) = name.to_str() else {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-unknown-or-pending-entry",
            ));
        };
        let Some(generation) = (!initializing)
            .then(|| pending_generation(name, STATE_NAME))
            .flatten()
        else {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-unknown-or-pending-entry",
            ));
        };
        if pending.is_some() {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-multiple-pending-publications",
            ));
        }
        let descriptor = openat(
            root,
            name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        let file = unsafe { File::from_raw_fd(descriptor) };
        let identity = safe_file_identity(&file)?;
        if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32
            || identity.links != 1
            || identity.mode & 0o777 != 0o600
            || identity.length > MAX_LEDGER_BYTES
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-pending-publication-unsafe",
            ));
        }
        pending = Some(PendingPublication {
            name: name.to_owned(),
            generation,
            identity,
        });
    }
    Ok(pending)
}
