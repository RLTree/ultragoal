fn initialize_execution_ledger(
    root_path: PathBuf,
    key: [u8; 32],
    binding: EvaluationExecutionBinding,
) -> Result<FileEvaluationExecutionLedger, EvaluationLedgerError> {
    let (root, root_identity) = open_safe_directory(&root_path)?;
    let lock = open_initialization_lock(&root)?;
    let lock_identity = safe_file_identity(&lock)?;
    let guard = FileLock::exclusive(&lock)?;
    require_named_lock_identity(&root, lock_identity)?;
    require_initialization_shape(&root)?;
    test_initialization_interruption(&root_path, "lock")?;
    let key_id = sha256(&key);
    let (anchor, record) =
        initialize_anchor(&root_path, &root, lock_identity, &key, &key_id, &binding)?;
    let anchor_authority = safe_file_identity(&anchor)?.authority();
    let anchor_observation = safe_file_identity(&anchor)?;
    let snapshot = authenticate_snapshot(
        SnapshotPayload {
            core: record.payload.core.clone(),
            anchor_observation,
            anchor_length: anchor_observation.length,
            anchor_head_sha256: record.head_sha256.clone(),
        },
        &key,
    )?;
    initialize_state(
        &root_path,
        &root,
        &snapshot,
        &key,
        lock_identity,
        anchor_authority,
        &binding,
    )?;
    sync_directory(&root)?;
    drop(guard);
    let ledger = FileEvaluationExecutionLedger {
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
    let guard = FileLock::exclusive(&ledger.lock)?;
    ledger.require_published_current(&snapshot)?;
    drop(guard);
    Ok(ledger)
}

fn open_initialization_lock(root: &File) -> Result<File, EvaluationLedgerError> {
    let names = read_directory_names(root)?;
    if names.iter().any(|name| name == OsStr::new(LOCK_NAME)) {
        return open_lock(root, false);
    }
    if !names.is_empty() {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-ambiguous",
        ));
    }
    open_lock(root, true).or_else(|_| open_lock(root, false))
}

fn require_initialization_shape(root: &File) -> Result<(), EvaluationLedgerError> {
    let allowed = [
        LOCK_NAME,
        ANCHOR_NAME,
        STATE_NAME,
        INITIAL_ANCHOR_NAME,
        INITIAL_STATE_NAME,
    ];
    if read_directory_names(root)?
        .iter()
        .any(|name| !allowed.iter().any(|allowed| name == OsStr::new(allowed)))
    {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-ambiguous",
        ));
    }
    let anchor = entry_exists(root, ANCHOR_NAME)?;
    let anchor_temp = entry_exists(root, INITIAL_ANCHOR_NAME)?;
    let state = entry_exists(root, STATE_NAME)?;
    let state_temp = entry_exists(root, INITIAL_STATE_NAME)?;
    if (state && !anchor) || (state_temp && !anchor) || (anchor_temp && (state || state_temp)) {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-ambiguous",
        ));
    }
    Ok(())
}

fn initialize_state(
    root_path: &Path,
    root: &File,
    expected: &AuthenticatedSnapshot,
    key: &[u8; 32],
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &EvaluationExecutionBinding,
) -> Result<(), EvaluationLedgerError> {
    if entry_exists(root, STATE_NAME)? {
        let named = open_initialization_file(root, STATE_NAME, false, false)?;
        require_state(
            &named,
            expected,
            key,
            lock_identity,
            anchor_authority,
            binding,
        )?;
        if entry_exists(root, INITIAL_STATE_NAME)? {
            let temporary = open_initialization_file(root, INITIAL_STATE_NAME, false, false)?;
            if safe_file_identity(&temporary)?.authority()
                != safe_file_identity(&named)?.authority()
            {
                return Err(EvaluationLedgerError::new(
                    "evaluation-ledger-initialization-ambiguous",
                ));
            }
            unlink_initialization_file(root, INITIAL_STATE_NAME)?;
        }
        return Ok(());
    }
    let scratch = if entry_exists(root, INITIAL_STATE_NAME)? {
        let scratch = open_initialization_file(root, INITIAL_STATE_NAME, false, false)?;
        if safe_file_identity(&scratch)?.length == 0 {
            unlink_initialization_file(root, INITIAL_STATE_NAME)?;
            open_initialization_file(root, INITIAL_STATE_NAME, true, false)?
        } else {
            require_state(
                &scratch,
                expected,
                key,
                lock_identity,
                anchor_authority,
                binding,
            )?;
            scratch
        }
    } else {
        open_initialization_file(root, INITIAL_STATE_NAME, true, false)?
    };
    if safe_file_identity(&scratch)?.length == 0 {
        write_initialization_json(&scratch, expected)?;
    }
    test_initialization_interruption(root_path, "state_temporary")?;
    publish_initialization_file(
        root,
        INITIAL_STATE_NAME,
        STATE_NAME,
        safe_file_identity(&scratch)?.authority(),
    )?;
    test_initialization_interruption(root_path, "state_named")?;
    Ok(())
}

fn require_state(
    file: &File,
    expected: &AuthenticatedSnapshot,
    key: &[u8; 32],
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &EvaluationExecutionBinding,
) -> Result<(), EvaluationLedgerError> {
    let observed: AuthenticatedSnapshot = read_initialization_json(file)?;
    verify_snapshot(
        &observed,
        key,
        &sha256(key),
        lock_identity,
        anchor_authority,
        binding,
    )?;
    if observed.head_sha256 != expected.head_sha256 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-initialization-ambiguous",
        ));
    }
    Ok(())
}
