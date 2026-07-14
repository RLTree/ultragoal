fn scan_anchor_journal(
    bytes: &[u8],
    key: &[u8; 32],
    key_id: &str,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &PromotionLedgerBinding,
) -> Result<ReviewAnchorJournalScan, PromotionLedgerError> {
    let mut records = Vec::new();
    let mut offset = 0_usize;
    let mut prior_head = sha256(ANCHOR_GENESIS);
    let mut generation = 0_u64;
    while offset < bytes.len() {
        if bytes.len() - offset < 8 {
            return Ok(ReviewAnchorJournalScan {
                records,
                complete_length: offset as u64,
                partial_tail: true,
            });
        }
        let length = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        if length == 0 || length > MAX_ANCHOR_RECORD_BYTES {
            return Err(PromotionLedgerError::new(
                "promotion-anchor-frame-length-invalid",
            ));
        }
        let end = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(length))
            .ok_or_else(|| PromotionLedgerError::new("promotion-anchor-frame-overflow"))?;
        if end > bytes.len() {
            return Ok(ReviewAnchorJournalScan {
                records,
                complete_length: offset as u64,
                partial_tail: true,
            });
        }
        let record: AuthenticatedReviewAnchorRecord =
            serde_json::from_slice(&bytes[offset + 8..end])
                .map_err(|_| PromotionLedgerError::new("promotion-anchor-record-malformed"))?;
        verify_anchor_record(&record, key)?;
        if record.payload.schema_version != "PromotionReviewAnchorRecord-v1"
            || record.payload.prior_anchor_head_sha256 != prior_head
            || record.payload.core.generation != generation
            || record.payload.core.key_id != key_id
            || record.payload.core.lock_identity != lock_identity
            || record.payload.core.anchor_authority != anchor_authority
            || record.payload.core.binding != *binding
            || !state_valid(&record.payload.core.state)
        {
            return Err(PromotionLedgerError::new(
                "promotion-anchor-record-binding-invalid",
            ));
        }
        prior_head = record.head_sha256.clone();
        generation = generation
            .checked_add(1)
            .ok_or_else(|| PromotionLedgerError::new("promotion-ledger-generation-overflow"))?;
        records.push(ScannedReviewAnchorRecord {
            end: end as u64,
            record,
        });
        offset = end;
    }
    Ok(ReviewAnchorJournalScan {
        records,
        complete_length: offset as u64,
        partial_tail: false,
    })
}

fn state_valid(state: &PromotionLedgerState) -> bool {
    match state {
        PromotionLedgerState::Ready => true,
        PromotionLedgerState::Issued {
            binding_sha256,
            attestation_sha256,
        } => super::valid_sha256(binding_sha256) && super::valid_sha256(attestation_sha256),
        PromotionLedgerState::Consumed {
            binding_sha256,
            review_id,
            attestation_sha256,
        } => {
            super::valid_sha256(binding_sha256)
                && super::valid_sha256(review_id)
                && super::valid_sha256(attestation_sha256)
        }
        PromotionLedgerState::RecoveryRequired { causal_code } => {
            super::valid_identifier(causal_code)
        }
    }
}

fn open_review_lock(root: &File, create: bool) -> Result<File, PromotionLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, LOCK_NAME, flags, 0o600).map_err(map_storage)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file).map_err(map_storage)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32 || identity.links != 1 {
        return Err(PromotionLedgerError::new("promotion-ledger-lock-unsafe"));
    }
    Ok(file)
}

fn open_review_anchor(root: &File, create: bool) -> Result<File, PromotionLedgerError> {
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    if create {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    let descriptor = openat(root, ANCHOR_NAME, flags, 0o600).map_err(map_storage)?;
    let file = unsafe { File::from_raw_fd(descriptor) };
    let identity = safe_file_identity(&file).map_err(map_storage)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32 || identity.links != 1 {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-authority-unsafe",
        ));
    }
    Ok(file)
}

fn require_named_review_lock_identity(
    root: &File,
    expected: FileIdentity,
) -> Result<(), PromotionLedgerError> {
    let current = open_review_lock(root, false)?;
    if safe_file_identity(&current).map_err(map_storage)? != expected {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-lock-descriptor-substituted",
        ));
    }
    Ok(())
}

fn require_named_review_anchor_authority(
    root: &File,
    expected: FileAuthorityIdentity,
) -> Result<(), PromotionLedgerError> {
    let current = open_review_anchor(root, false)?;
    if safe_file_identity(&current)
        .map_err(map_storage)?
        .authority()
        != expected
    {
        return Err(PromotionLedgerError::new(
            "promotion-anchor-descriptor-substituted",
        ));
    }
    Ok(())
}

fn read_anchor_bytes(anchor: &File) -> Result<Vec<u8>, PromotionLedgerError> {
    use std::os::unix::fs::FileExt;

    let identity = safe_file_identity(anchor).map_err(map_storage)?;
    if identity.length == 0 || identity.length > MAX_ANCHOR_JOURNAL_BYTES {
        return Err(PromotionLedgerError::new("promotion-anchor-length-invalid"));
    }
    let mut bytes = vec![0_u8; identity.length as usize];
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let read = anchor
            .read_at(&mut bytes[offset..], offset as u64)
            .map_err(|_| PromotionLedgerError::new("promotion-anchor-read-failed"))?;
        if read == 0 {
            return Err(PromotionLedgerError::new("promotion-anchor-read-truncated"));
        }
        offset += read;
    }
    Ok(bytes)
}
