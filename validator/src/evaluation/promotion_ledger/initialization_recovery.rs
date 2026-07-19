fn initialize_promotion_ledger(
    root_path: PathBuf,
    key: [u8; 32],
    binding: PromotionLedgerBinding,
) -> Result<FilePromotionReviewLedger, PromotionLedgerError> {
    validate_binding(&binding)?;
    let (root, root_identity) = open_safe_directory(&root_path)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-root-unsafe"))?;
    let lock = open_promotion_initialization_lock(&root)?;
    let lock_identity = safe_file_identity(&lock).map_err(map_storage)?;
    let _guard = FileLock::exclusive(&lock)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
    require_named_review_lock_identity(&root, lock_identity)?;
    require_promotion_initialization_shape(&root)?;
    test_promotion_initialization_interruption(&root_path, "lock")?;
    let key_id = sha256(&key);
    let (anchor, record) =
        initialize_promotion_anchor(&root_path, &root, lock_identity, &key, &key_id, &binding)?;
    let anchor_authority = safe_file_identity(&anchor)
        .map_err(map_storage)?
        .authority();
    let observed_anchor = safe_file_identity(&anchor).map_err(map_storage)?;
    let snapshot = authenticate_snapshot(
        ReviewSnapshotPayload {
            core: record.payload.core.clone(),
            anchor_observation: observed_anchor,
            anchor_length: observed_anchor.length,
            anchor_head_sha256: record.head_sha256.clone(),
        },
        &key,
    )?;
    initialize_promotion_state(
        &root_path,
        &root,
        &snapshot,
        &key,
        lock_identity,
        anchor_authority,
        &binding,
    )?;
    sync_directory(&root).map_err(map_storage)?;
    drop(_guard);
    let ledger = FilePromotionReviewLedger {
        root_path,
        root,
        root_identity,
        lock,
        lock_identity,
        anchor,
        anchor_authority,
        key,
        key_id,
        binding,
        expected_head: snapshot.head_sha256.clone(),
    };
    let _guard = FileLock::exclusive(&ledger.lock)
        .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
    ledger.require_published_current(&snapshot)?;
    drop(_guard);
    Ok(ledger)
}

fn open_promotion_initialization_lock(root: &File) -> Result<File, PromotionLedgerError> {
    let names = read_directory_names(root).map_err(map_storage)?;
    if names
        .iter()
        .any(|name| name == std::ffi::OsStr::new(LOCK_NAME))
    {
        return open_review_lock(root, false);
    }
    if !names.is_empty() {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-ambiguous",
        ));
    }
    open_review_lock(root, true).or_else(|_| open_review_lock(root, false))
}

fn require_promotion_initialization_shape(root: &File) -> Result<(), PromotionLedgerError> {
    let allowed = [
        LOCK_NAME,
        ANCHOR_NAME,
        STATE_NAME,
        INITIAL_ANCHOR_NAME,
        INITIAL_STATE_NAME,
    ];
    if read_directory_names(root)
        .map_err(map_storage)?
        .iter()
        .any(|name| {
            !allowed
                .iter()
                .any(|allowed| name == std::ffi::OsStr::new(allowed))
        })
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-ambiguous",
        ));
    }
    let anchor = entry_exists(root, ANCHOR_NAME).map_err(map_storage)?;
    let anchor_temp = entry_exists(root, INITIAL_ANCHOR_NAME).map_err(map_storage)?;
    let state = entry_exists(root, STATE_NAME).map_err(map_storage)?;
    let state_temp = entry_exists(root, INITIAL_STATE_NAME).map_err(map_storage)?;
    if (state && !anchor) || (state_temp && !anchor) || (anchor_temp && (state || state_temp)) {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-ambiguous",
        ));
    }
    Ok(())
}

fn promotion_genesis_core(
    key_id: &str,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: PromotionLedgerBinding,
) -> ReviewSnapshotCore {
    ReviewSnapshotCore {
        schema_version: "PromotionReviewLedger-v1".to_owned(),
        generation: 0,
        previous_head_sha256: sha256(b"promotion-review-genesis"),
        key_id: key_id.to_owned(),
        lock_identity,
        anchor_authority,
        binding,
        state: PromotionLedgerState::Ready,
    }
}

fn initialize_promotion_anchor(
    root_path: &Path,
    root: &File,
    lock_identity: FileIdentity,
    key: &[u8; 32],
    key_id: &str,
    binding: &PromotionLedgerBinding,
) -> Result<(File, AuthenticatedReviewAnchorRecord), PromotionLedgerError> {
    if entry_exists(root, ANCHOR_NAME).map_err(map_storage)? {
        let named = open_promotion_initialization_file(root, ANCHOR_NAME, false, true)?;
        let record = require_promotion_genesis(&named, key, key_id, lock_identity, binding)?;
        cleanup_same_initialization_file(root, INITIAL_ANCHOR_NAME, &named, true)?;
        return Ok((open_review_anchor(root, false)?, record));
    }
    let scratch = recover_or_create_promotion_initialization_file(root, INITIAL_ANCHOR_NAME, true)?;
    let authority = safe_file_identity(&scratch)
        .map_err(map_storage)?
        .authority();
    let record = if safe_file_identity(&scratch).map_err(map_storage)?.length == 0 {
        let core = promotion_genesis_core(key_id, lock_identity, authority, binding.clone());
        let record = authenticate_anchor_record(
            ReviewAnchorRecordPayload {
                schema_version: "PromotionReviewAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: sha256(ANCHOR_GENESIS),
                core,
            },
            key,
        )?;
        append_anchor_record(&scratch, &record)?;
        record
    } else {
        require_promotion_genesis(&scratch, key, key_id, lock_identity, binding)?
    };
    test_promotion_initialization_interruption(root_path, "anchor_temporary")?;
    publish_promotion_initialization_file(root, INITIAL_ANCHOR_NAME, ANCHOR_NAME, authority)?;
    test_promotion_initialization_interruption(root_path, "anchor_named")?;
    Ok((open_review_anchor(root, false)?, record))
}

fn initialize_promotion_state(
    root_path: &Path,
    root: &File,
    expected: &AuthenticatedReviewSnapshot,
    key: &[u8; 32],
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &PromotionLedgerBinding,
) -> Result<(), PromotionLedgerError> {
    if entry_exists(root, STATE_NAME).map_err(map_storage)? {
        let named = open_promotion_initialization_file(root, STATE_NAME, false, false)?;
        require_promotion_state(
            &named,
            expected,
            key,
            lock_identity,
            anchor_authority,
            binding,
        )?;
        return cleanup_same_initialization_file(root, INITIAL_STATE_NAME, &named, false);
    }
    let scratch = recover_or_create_promotion_initialization_file(root, INITIAL_STATE_NAME, false)?;
    if safe_file_identity(&scratch).map_err(map_storage)?.length == 0 {
        write_promotion_initialization_json(&scratch, expected)?;
    }
    let scratch_authority = safe_file_identity(&scratch)
        .map_err(map_storage)?
        .authority();
    test_promotion_initialization_interruption(root_path, "state_reopen")?;
    // A fresh write leaves its descriptor at EOF. Reopen the exact named
    // scratch file so durable verification starts at offset zero while
    // retaining full identity and authenticated-content checks.
    let verified = open_promotion_initialization_file(root, INITIAL_STATE_NAME, false, false)?;
    let verified_authority = safe_file_identity(&verified)
        .map_err(map_storage)?
        .authority();
    if verified_authority != scratch_authority {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-file-changed",
        ));
    }
    require_promotion_state(
        &verified,
        expected,
        key,
        lock_identity,
        anchor_authority,
        binding,
    )?;
    drop(verified);
    test_promotion_initialization_interruption(root_path, "state_temporary")?;
    publish_promotion_initialization_file(root, INITIAL_STATE_NAME, STATE_NAME, scratch_authority)?;
    test_promotion_initialization_interruption(root_path, "state_named")
}
