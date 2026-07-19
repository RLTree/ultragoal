fn genesis_core(
    key_id: &str,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: EvaluationExecutionBinding,
) -> SnapshotCore {
    SnapshotCore {
        schema_version: "EvaluationExecutionLedger-v1".to_owned(),
        generation: 0,
        previous_head_sha256: sha256(b"evaluation-ledger-genesis"),
        key_id: key_id.to_owned(),
        lock_identity,
        anchor_authority,
        binding,
        reservation_id_sha256: None,
        state: EvaluationLedgerState::Initialized,
    }
}

fn initialize_anchor(
    root_path: &Path,
    root: &File,
    lock_identity: FileIdentity,
    key: &[u8; 32],
    key_id: &str,
    binding: &EvaluationExecutionBinding,
) -> Result<(File, AuthenticatedAnchorRecord), EvaluationLedgerError> {
    if entry_exists(root, ANCHOR_NAME)? {
        let named = open_initialization_file(root, ANCHOR_NAME, false, true)?;
        let record = require_genesis(&named, key, key_id, lock_identity, binding)?;
        if entry_exists(root, INITIAL_ANCHOR_NAME)? {
            let temporary = open_initialization_file(root, INITIAL_ANCHOR_NAME, false, true)?;
            if safe_file_identity(&temporary)?.authority()
                != safe_file_identity(&named)?.authority()
            {
                return Err(EvaluationLedgerError::new(
                    "evaluation-ledger-initialization-ambiguous",
                ));
            }
            unlink_initialization_file(root, INITIAL_ANCHOR_NAME)?;
            sync_directory(root)?;
        }
        return Ok((open_anchor(root, false)?, record));
    }
    let scratch = if entry_exists(root, INITIAL_ANCHOR_NAME)? {
        let scratch = open_initialization_file(root, INITIAL_ANCHOR_NAME, false, true)?;
        if safe_file_identity(&scratch)?.length == 0 {
            unlink_initialization_file(root, INITIAL_ANCHOR_NAME)?;
            open_initialization_file(root, INITIAL_ANCHOR_NAME, true, true)?
        } else {
            scratch
        }
    } else {
        open_initialization_file(root, INITIAL_ANCHOR_NAME, true, true)?
    };
    let authority = safe_file_identity(&scratch)?.authority();
    let record = if safe_file_identity(&scratch)?.length == 0 {
        let core = genesis_core(key_id, lock_identity, authority, binding.clone());
        let record = authenticate_anchor_record(
            AnchorRecordPayload {
                schema_version: "EvaluationExecutionAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: sha256(ANCHOR_GENESIS),
                core,
            },
            key,
        )?;
        append_anchor_record(&scratch, &record)?;
        record
    } else {
        require_genesis(&scratch, key, key_id, lock_identity, binding)?
    };
    test_initialization_interruption(root_path, "anchor_temporary")?;
    publish_initialization_file(root, INITIAL_ANCHOR_NAME, ANCHOR_NAME, authority)?;
    test_initialization_interruption(root_path, "anchor_named")?;
    Ok((open_anchor(root, false)?, record))
}

fn require_genesis(
    anchor: &File,
    key: &[u8; 32],
    key_id: &str,
    lock_identity: FileIdentity,
    binding: &EvaluationExecutionBinding,
) -> Result<AuthenticatedAnchorRecord, EvaluationLedgerError> {
    let authority = safe_file_identity(anchor)?.authority();
    let bytes = read_file_bytes(anchor, MAX_ANCHOR_JOURNAL_BYTES)?;
    let scan = scan_anchor_journal(&bytes, key, key_id, lock_identity, authority, binding)?;
    let [only] = scan.records.as_slice() else {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-ambiguous",
        ));
    };
    if scan.partial_tail
        || scan.complete_length != safe_file_identity(anchor)?.length
        || only.record.payload.core
            != genesis_core(key_id, lock_identity, authority, binding.clone())
    {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-ambiguous",
        ));
    }
    Ok(only.record.clone())
}
