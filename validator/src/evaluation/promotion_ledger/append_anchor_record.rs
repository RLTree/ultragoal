fn append_anchor_record(
    anchor: &File,
    record: &AuthenticatedReviewAnchorRecord,
) -> Result<(FileIdentity, u64), PromotionLedgerError> {
    let bytes = serde_json::to_vec(record)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-serialization-failed"))?;
    if bytes.is_empty() || bytes.len() > MAX_ANCHOR_RECORD_BYTES {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-record-length-invalid",
        ));
    }
    let before = safe_file_identity(anchor).map_err(map_storage)?;
    let frame_length = 8_u64
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| PromotionLedgerError::new("promotion-anchor-frame-overflow"))?;
    let end = before
        .length
        .checked_add(frame_length)
        .filter(|end| *end <= MAX_ANCHOR_JOURNAL_BYTES)
        .ok_or_else(|| PromotionLedgerError::new("promotion-anchor-journal-full"))?;
    let header = (bytes.len() as u64).to_be_bytes();
    write_all_at(anchor, &header, before.length)?;
    write_all_at(anchor, &bytes, before.length + header.len() as u64)?;
    anchor
        .sync_all()
        .map_err(|_| PromotionLedgerError::new("promotion-anchor-fsync-failed"))?;
    let after = safe_file_identity(anchor).map_err(map_storage)?;
    if after.authority() != before.authority() || after.length != end {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-changed-during-append",
        ));
    }
    Ok((after, end))
}

fn write_all_at(anchor: &File, bytes: &[u8], offset: u64) -> Result<(), PromotionLedgerError> {
    use std::os::unix::fs::FileExt;

    let mut written = 0_usize;
    while written < bytes.len() {
        let count = anchor
            .write_at(&bytes[written..], offset + written as u64)
            .map_err(|_| PromotionLedgerError::new("promotion-anchor-write-failed"))?;
        if count == 0 {
            return Err(PromotionLedgerError::new("promotion-anchor-write-failed"));
        }
        written += count;
    }
    Ok(())
}

fn ensure_entries(
    path: &Path,
    root: &File,
    initializing: bool,
) -> Result<Option<PendingReviewPublication>, PromotionLedgerError> {
    let allowed = [ANCHOR_NAME, LOCK_NAME, STATE_NAME];
    test_directory_scan_pause(path);
    let entries = read_directory_names(root).map_err(map_storage)?;
    let mut pending = None;
    for name in entries {
        if !initializing
            && allowed
                .iter()
                .any(|allowed| name == std::ffi::OsStr::new(allowed))
        {
            continue;
        }
        let Some(name) = name.to_str() else {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-unknown-or-pending-entry",
            ));
        };
        let Some(generation) = (!initializing)
            .then(|| pending_generation(name, STATE_NAME))
            .flatten()
        else {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-unknown-or-pending-entry",
            ));
        };
        if pending.is_some() {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-multiple-pending-publications",
            ));
        }
        let descriptor = openat(
            root,
            name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )
        .map_err(map_storage)?;
        let file = unsafe { File::from_raw_fd(descriptor) };
        let identity = safe_file_identity(&file).map_err(map_storage)?;
        if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32
            || identity.links != 1
            || identity.mode & 0o777 != 0o600
            || identity.length > MAX_LEDGER_BYTES
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-pending-publication-unsafe",
            ));
        }
        pending = Some(PendingReviewPublication {
            name: name.to_owned(),
            generation,
            identity,
        });
    }
    Ok(pending)
}

fn pending_generation(name: &str, destination: &str) -> Option<u64> {
    let remainder = name.strip_prefix(&format!(".{destination}.pending."))?;
    let (pid, generation) = remainder.split_once('.')?;
    if pid.is_empty()
        || generation.is_empty()
        || generation.contains('.')
        || !pid.bytes().all(|byte| byte.is_ascii_digit())
        || !generation.bytes().all(|byte| byte.is_ascii_digit())
        || (pid.len() > 1 && pid.starts_with('0'))
        || (generation.len() > 1 && generation.starts_with('0'))
        || pid.parse::<u32>().ok()? == 0
    {
        return None;
    }
    generation.parse().ok()
}

fn validate_pending_generation(
    pending: Option<&PendingReviewPublication>,
    snapshot: &AuthenticatedReviewSnapshot,
) -> Result<(), PromotionLedgerError> {
    if pending.is_some_and(|pending| pending.generation != snapshot.payload.core.generation) {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-pending-generation-invalid",
        ));
    }
    Ok(())
}

fn remove_pending_publication(
    root: &File,
    pending: &PendingReviewPublication,
) -> Result<(), PromotionLedgerError> {
    let descriptor = openat(
        root,
        &pending.name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        0,
    )
    .map_err(map_storage)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    if safe_file_identity(&file).map_err(map_storage)? != pending.identity {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-pending-publication-substituted",
        ));
    }
    let name = std::ffi::CString::new(pending.name.as_str())
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-component-invalid"))?;
    if unsafe { libc::unlinkat(root.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-pending-publication-remove-failed",
        ));
    }
    Ok(())
}

fn map_storage(_: super::EvaluationLedgerError) -> PromotionLedgerError {
    PromotionLedgerError::new("promotion-ledger-storage-failed")
}
