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
    pending: Option<&PendingPublication>,
    snapshot: &AuthenticatedSnapshot,
) -> Result<(), EvaluationLedgerError> {
    if pending.is_some_and(|pending| pending.generation != snapshot.payload.core.generation) {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-pending-generation-invalid",
        ));
    }
    Ok(())
}

fn remove_pending_publication(
    root: &File,
    pending: &PendingPublication,
) -> Result<(), EvaluationLedgerError> {
    let descriptor = openat(
        root,
        &pending.name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        0,
    )?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    if safe_file_identity(&file)? != pending.identity {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-pending-publication-substituted",
        ));
    }
    let name = c_string(&pending.name)?;
    if unsafe { libc::unlinkat(root.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-pending-publication-remove-failed",
        ));
    }
    Ok(())
}

pub(super) fn read_json_file<T: for<'de> Deserialize<'de>>(
    root: &File,
    name: &str,
) -> Result<T, EvaluationLedgerError> {
    let descriptor = openat(
        root,
        name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        0,
    )?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file)?;
    let length = file
        .metadata()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-stat-failed"))?
        .len();
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32
        || identity.links != 1
        || identity.mode & 0o022 != 0
        || length == 0
        || length > MAX_LEDGER_BYTES
    {
        return Err(EvaluationLedgerError::new("evaluation-ledger-file-unsafe"));
    }
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(MAX_LEDGER_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-read-failed"))?;
    if bytes.len() as u64 != length || bytes.len() as u64 > MAX_LEDGER_BYTES {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-truncated-or-oversized",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-malformed"))
}

fn read_file_bytes(file: &File, maximum: u64) -> Result<Vec<u8>, EvaluationLedgerError> {
    use std::os::unix::fs::FileExt;

    let identity = safe_file_identity(file)?;
    if identity.length == 0 || identity.length > maximum {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-length-invalid",
        ));
    }
    let mut bytes = vec![0_u8; identity.length as usize];
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let read = file
            .read_at(&mut bytes[offset..], offset as u64)
            .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-read-failed"))?;
        if read == 0 {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-read-truncated",
            ));
        }
        offset += read;
    }
    Ok(bytes)
}

fn append_anchor_record(
    anchor: &File,
    record: &AuthenticatedAnchorRecord,
) -> Result<(FileIdentity, u64), EvaluationLedgerError> {
    use std::os::unix::fs::FileExt;

    let bytes = serde_json::to_vec(record)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    if bytes.is_empty() || bytes.len() > MAX_ANCHOR_RECORD_BYTES {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-record-length-invalid",
        ));
    }
    let before = safe_file_identity(anchor)?;
    let frame_length = 8_u64
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| EvaluationLedgerError::new("evaluation-anchor-frame-overflow"))?;
    let end = before
        .length
        .checked_add(frame_length)
        .filter(|end| *end <= MAX_ANCHOR_JOURNAL_BYTES)
        .ok_or_else(|| EvaluationLedgerError::new("evaluation-anchor-journal-full"))?;
    let header = (bytes.len() as u64).to_be_bytes();
    let mut written = 0_usize;
    while written < header.len() {
        let count = anchor
            .write_at(&header[written..], before.length + written as u64)
            .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-write-failed"))?;
        if count == 0 {
            return Err(EvaluationLedgerError::new("evaluation-anchor-write-failed"));
        }
        written += count;
    }
    written = 0;
    while written < bytes.len() {
        let count = anchor
            .write_at(
                &bytes[written..],
                before.length + header.len() as u64 + written as u64,
            )
            .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-write-failed"))?;
        if count == 0 {
            return Err(EvaluationLedgerError::new("evaluation-anchor-write-failed"));
        }
        written += count;
    }
    anchor
        .sync_all()
        .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-fsync-failed"))?;
    let after = safe_file_identity(anchor)?;
    if after.authority() != before.authority() || after.length != end {
        return Err(EvaluationLedgerError::new(
            "evaluation-anchor-changed-during-append",
        ));
    }
    Ok((after, end))
}
